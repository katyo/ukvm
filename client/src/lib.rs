mod result;

#[cfg(feature = "dbus")]
mod dbus;

pub use tracing as log;

pub use ukvm_core::{ButtonId, LedId};

use futures_util::stream::Stream;

#[cfg(any(feature = "dbus", feature = "http"))]
pub use ukvm_core::Addr;

#[cfg(feature = "dbus")]
pub use ukvm_core::DBusAddr;

#[cfg(feature = "http")]
pub use ukvm_core::{HttpAddr, SocketInput, SocketOutput};

#[cfg(feature = "dbus")]
pub use dbus::DBusClient;

//#[cfg(feature = "http")]
//pub use http::{HttpConn};

pub use result::{Error, Result};

#[derive(Clone, Debug)]
pub enum ClientEvent {
    Button { id: ButtonId, state: bool },
    Led { id: LedId, state: bool },
}

#[async_trait::async_trait]
trait GenericClient {
    fn buttons(&self) -> Vec<ButtonId>;
    fn button_state(&self, id: ButtonId) -> Result<bool>;
    async fn set_button_state(&self, id: ButtonId, state: bool) -> Result<()>;

    fn leds(&self) -> Vec<LedId>;
    fn led_state(&self, id: LedId) -> Result<bool>;

    fn events(&self) -> Box<dyn Stream<Item = ClientEvent> + 'static>;
}

pub struct Client {
    inner: Box<dyn GenericClient>,
}

impl Client {
    pub async fn open(addr: &Addr) -> Result<Self> {
        let inner = match addr {
            #[cfg(feature = "dbus")]
            Addr::DBus(addr) => Box::new(DBusClient::open(addr).await?) as Box<dyn GenericClient>,
            #[cfg(feature = "http")]
            Addr::Http(_addr) => todo!(),
        };

        Ok(Self { inner })
    }

    pub fn buttons(&self) -> Vec<ButtonId> {
        self.inner.buttons()
    }

    pub fn button_state(&self, id: ButtonId) -> Result<bool> {
        self.inner.button_state(id)
    }

    pub async fn set_button_state(&self, id: ButtonId, state: bool) -> Result<()> {
        self.inner.set_button_state(id, state).await
    }

    pub fn leds(&self) -> Vec<LedId> {
        self.inner.leds()
    }

    pub fn led_state(&self, id: LedId) -> Result<bool> {
        self.inner.led_state(id)
    }

    pub fn events(&self) -> Box<dyn Stream<Item = ClientEvent> + 'static> {
        self.inner.events()
    }
}
