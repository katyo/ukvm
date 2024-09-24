use crate::{ButtonId, LedId, DBusAddr, Result, GenericClient, ClientEvent, Stream, log};
use zbus::{proxy, Connection, proxy::Proxy};
use std::{collections::HashMap, sync::{atomic::{Ordering, AtomicBool}, Arc}};
use tokio::sync::broadcast::{channel, Sender};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

/// Button handling interface
#[proxy(interface = "org.ukvm.Button", default_service = "org.ukvm.Control")]
pub trait Button {
    /// Button id
    #[zbus(property)]
    fn id(&self) -> zbus::Result<ButtonId>;

    /// Current state
    #[zbus(property)]
    fn state(&self) -> zbus::Result<bool>;

    /// Change state
    #[zbus(property)]
    fn set_state(&self, state: bool) -> zbus::Result<()>;
}

/// LED handling interface
#[proxy(interface = "org.ukvm.Led", default_service = "org.ukvm.Control")]
pub trait Led {
    /// Led id
    #[zbus(property)]
    fn id(&self) -> zbus::Result<LedId>;

    /// Current state
    #[zbus(property)]
    fn state(&self) -> zbus::Result<bool>;
}

struct Button {
    state: Arc<AtomicBool>,
    proxy: ButtonProxy<'static>,
}

struct Led {
    state: Arc<AtomicBool>,
    proxy: LedProxy<'static>,
}

pub struct DBusClient {
    buttons: HashMap<ButtonId, Button>,
    leds: HashMap<LedId, Led>,
    events: Sender<ClientEvent>,
}

impl DBusClient {
    pub async fn open(_addr: &DBusAddr) -> Result<Self> {
        let connection = Connection::system().await?;

        log::info!("Init DBus client");

        let button_names = Self::list_nodes(&connection,
                                            "org.ukvm.Control",
                                            "/org/ukvm/button").await.unwrap_or_default();

        let led_names = Self::list_nodes(&connection,
                                         "org.ukvm.Control",
                                         "/org/ukvm/led").await.unwrap_or_default();

        let (sender, _) = channel(16);
        let events = sender.clone();

        let mut buttons = HashMap::new();

        for name in button_names {
            if let Ok(id) = name.parse() {
                log::info!("Add button {id}");

                let proxy = ButtonProxy::builder(&connection)
                    .path(format!("/org/ukvm/button/{id}"))?
                    .cache_properties(zbus::CacheProperties::Yes)
                    .build()
                    .await?;

                let state = proxy.state().await?;
                let state = Arc::new(AtomicBool::new(state));

                buttons.insert(id, Button { state, proxy });
            }
        }

        let mut leds = HashMap::new();

        for name in led_names {
            if let Ok(id) = name.parse() {
                log::info!("Add LED {id}");

                let proxy = LedProxy::builder(&connection)
                    .path(format!("/org/ukvm/led/{id}"))?
                    .cache_properties(zbus::CacheProperties::Yes)
                    .build()
                    .await?;

                let state = proxy.state().await?;
                let state = Arc::new(AtomicBool::new(state));

                leds.insert(id, Led { state, proxy });
            }
        }

        Ok(Self { buttons, leds, events })
    }

    async fn list_nodes(connection: &Connection, destination: impl AsRef<str>, path: impl AsRef<str>) -> Result<Vec<String>> {
        use quick_xml::{reader::Reader, events::Event};

        let path = path.as_ref();

        let proxy = Proxy::new(connection,
                               destination.as_ref(),
                               path,
                               "org.freedesktop.DBus.Introspectable").await?;

        let xml = proxy.introspect().await?;

        let mut reader = Reader::from_str(&xml);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut nodes = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Err(e) => Err(format!("XML error at {}: {:?}", reader.buffer_position(), e))?,
                Ok(Event::Eof) => break,
                Ok(Event::Start(e) | Event::Empty(e)) => {
                    if e.name().as_ref() == b"node" {
                        if let Ok(Some(name)) = e.try_get_attribute(b"name") {
                            if let Ok(name) = core::str::from_utf8(&name.value) {
                                nodes.push(name.into());
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        Ok(nodes)
    }
}

#[async_trait::async_trait]
impl GenericClient for DBusClient {
    fn buttons(&self) -> Vec<ButtonId> {
        self.buttons.keys().cloned().collect()
    }

    fn button_state(&self, id: ButtonId) -> Result<bool> {
        let button = self.buttons.get(&id).ok_or_else(|| format!("No button {id}"))?;
        Ok(button.state.load(Ordering::SeqCst))
    }

    async fn set_button_state(&self, id: ButtonId, state: bool) -> Result<()> {
        let button = self.buttons.get(&id).ok_or_else(|| format!("No button {id}"))?;
        Ok(button.proxy.set_state(state).await?)
    }

    fn leds(&self) -> Vec<LedId> {
        self.leds.keys().cloned().collect()
    }

    fn led_state(&self, id: LedId) -> Result<bool> {
        let led = self.leds.get(&id).ok_or_else(|| format!("No LED {id}"))?;
        Ok(led.state.load(Ordering::SeqCst))
    }

    fn events(&self) -> Box<dyn Stream<Item = ClientEvent> + 'static> {
        Box::new(BroadcastStream::new(self.events.subscribe()).filter_map(|res| res.ok()))
    }
}
