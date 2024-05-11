use crate::{log, Error, HttpAddr, HttpBindAddr, Result, Server, SocketInput, SocketOutput};
use core::pin::Pin;
use futures_util::{
    sink::SinkExt,
    stream::{once, select, select_all, Stream, StreamExt},
};
use std::sync::Arc;
use tokio::{
    fs::{metadata, remove_file},
    net::UnixListener,
    spawn,
    sync::Semaphore,
};
use tokio_stream::wrappers::{ReceiverStream, UnixListenerStream, WatchStream};

#[cfg(not(feature = "postcard"))]
use serde_json::{from_slice, to_vec};

#[cfg(feature = "postcard")]
use postcard::{from_bytes as from_slice, to_stdvec as to_vec};

impl warp::reject::Reject for Error {}

impl Server {
    pub async fn spawn_http(&self, addr: &HttpBindAddr, stop: &Arc<Semaphore>) -> Result<()> {
        use warp::Filter;

        let stop = stop.clone();

        let server_ref = self.downgrade();
        let server = warp::any().and_then(move || {
            let server_ref = server_ref.clone();
            async move { Ok::<_, warp::Rejection>(server_ref.upgrade()?) }
        });

        #[cfg(not(feature = "web"))]
        let index = warp::path::end().and(warp::get()).map(|| {
            warp::http::Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .body(include_str!("index.html"))
        });

        #[cfg(feature = "web")]
        let index = include!(concat!(env!("OUT_DIR"), "/web.rs"));

        let socket = warp::path("socket")
            .and(warp::path::end())
            .and(warp::ws())
            .and(server.clone())
            .map(|ws: warp::ws::Ws, server: Server| {
                ws.on_upgrade(move |socket| async move {
                    let (mut socket_sender, mut socket_receiver) = socket.split();

                    spawn({
                        let server = server.clone();
                        async move {
                            let mut stream = server.create_socket_output();
                            while let Some(res) = stream.next().await {
                                let msg = match to_vec(&res) {
                                    Ok(res) => warp::ws::Message::binary(res),
                                    Err(error) => {
                                        log::error!("Error when encoding message: {}", error);
                                        continue;
                                    }
                                };
                                if let Err(error) = socket_sender.feed(msg).await {
                                    log::warn!("Error when sending message: {}", error);
                                    break;
                                }
                            }
                        }
                    });

                    let server = server.downgrade();

                    while let Some(req) = socket_receiver.next().await {
                        let msg = match req {
                            Ok(msg) => msg,
                            Err(error) => {
                                log::warn!("Error when receiving message: {}", error);
                                continue;
                            }
                        };
                        if msg.is_text() {
                            let req = match from_slice(msg.as_bytes()) {
                                Ok(req) => req,
                                Err(error) => {
                                    log::warn!("Error when parsing message: {}", error);
                                    continue;
                                }
                            };

                            let server = if let Ok(server) = server.upgrade() {
                                server
                            } else {
                                break;
                            };

                            if let Err(error) = server.process_socket_input(req).await {
                                log::warn!("Error when processing input: {}", error);
                            }
                        }
                    }
                })
            });

        let http_server = warp::serve(index.or(socket));

        let tls = &addr.tls;

        match &addr.addr {
            HttpAddr::Addr(addr) => {
                log::debug!("Starting {addr}");

                #[cfg(feature = "tls")]
                if let Some(tls) = tls {
                    let http_server = http_server.tls().cert_path(&tls.cert).key_path(&tls.key);
                    let http_server = if let Some(auth) = &tls.auth {
                        http_server.client_auth_required_path(&auth)
                    } else {
                        http_server
                    };

                    let (addr, future) =
                        http_server.bind_with_graceful_shutdown(*addr, async move {
                            log::debug!("Await signal to stop");
                            let lock = stop.acquire().await;
                            log::debug!("Received stop signal");
                            drop(lock);
                        });

                    spawn(async move {
                        log::info!("Started {addr}");
                        future.await;
                        log::info!("Stopped {addr}");
                    });

                    return Ok(());
                }

                let (addr, future) = http_server.bind_with_graceful_shutdown(*addr, async move {
                    log::debug!("Await signal to stop");
                    let lock = stop.acquire().await;
                    log::debug!("Received stop signal");
                    drop(lock);
                });

                spawn(async move {
                    log::info!("Started {addr}");
                    future.await;
                    log::info!("Stopped {addr}");
                });
            }
            HttpAddr::Path(path) => {
                use std::os::unix::fs::FileTypeExt;

                log::debug!("Starting {}", path.display());

                // Remove socket file if exists
                if let Ok(meta) = metadata(&path).await {
                    if meta.file_type().is_socket() {
                        remove_file(&path).await?;
                    }
                }

                let future = http_server.serve_incoming_with_graceful_shutdown(
                    UnixListenerStream::new(UnixListener::bind(path)?),
                    async move {
                        log::debug!("Await signal to stop");
                        let lock = stop.acquire().await;
                        log::debug!("Stopped");
                        drop(lock);
                    },
                );

                let path = path.clone();

                spawn(async move {
                    log::info!("Started {}", path.display());
                    future.await;
                    log::info!("Stopped {}", path.display());
                    // Don't forget remove socket file
                    let _ = remove_file(&path).await;
                });
            }
        }

        Ok(())
    }

    fn create_socket_state(&self) -> SocketOutput {
        let leds = self
            .leds()
            .iter()
            .map(|(id, obj)| (*id, obj.state()))
            .collect();

        let buttons = self
            .buttons()
            .iter()
            .map(|(id, obj)| (*id, obj.state()))
            .collect();

        #[cfg(feature = "hid")]
        let keyboard = self
            .hid()
            .and_then(|hid| hid.keyboard())
            .map(|keyboard| keyboard.get_state());

        #[cfg(feature = "hid")]
        let mouse = self
            .hid()
            .and_then(|hid| hid.mouse())
            .map(|mouse| mouse.get_state());

        SocketOutput::State {
            leds,
            buttons,
            #[cfg(feature = "hid")]
            keyboard,
            #[cfg(feature = "hid")]
            mouse,
        }
    }

    fn create_socket_output(&self) -> impl Stream<Item = SocketOutput> {
        let button_events = select_all(self.buttons().iter().map(|(id, obj)| {
            let button = *id;
            WatchStream::new(obj.watch()).map(move |state| SocketOutput::Button { button, state })
        }));

        let led_events = select_all(self.leds().iter().map(|(id, obj)| {
            let led = *id;
            WatchStream::new(obj.watch()).map(move |state| SocketOutput::Led { led, state })
        }));

        let events = select(button_events, led_events);

        let events = select(
            events,
            once({
                let state = self.create_socket_state();
                async move { state }
            }),
        );

        #[cfg(feature = "hid")]
        let events = {
            let mut events = Box::pin(events) as Pin<Box<dyn Stream<Item = SocketOutput> + Send>>;

            if let Some(keyboard) = self.hid().and_then(|hid| hid.keyboard()) {
                let key_events = ReceiverStream::new(keyboard.watch_keys()).map(|change| {
                    SocketOutput::KeyboardKey {
                        key: *change,
                        state: change.state(),
                    }
                });

                let led_events = ReceiverStream::new(keyboard.watch_leds()).map(|change| {
                    SocketOutput::KeyboardLed {
                        led: *change,
                        state: change.state(),
                    }
                });

                let keyboard_events = select(key_events, led_events);

                events = Box::pin(select(events, keyboard_events))
            }

            if let Some(mouse) = self.hid().and_then(|hid| hid.mouse()) {
                use crate::hid::MouseStateChange;

                let mouse_events =
                    ReceiverStream::new(mouse.watch_state()).map(|change| match change {
                        MouseStateChange::Button(change) => SocketOutput::MouseButton {
                            button: *change,
                            state: change.state(),
                        },
                        MouseStateChange::Pointer(change) => SocketOutput::MousePointer {
                            x: change.0,
                            y: change.1,
                        },
                        MouseStateChange::Wheel(change) => {
                            SocketOutput::MouseWheel { wheel: *change }
                        }
                    });

                events = Box::pin(select(events, mouse_events))
            }

            events
        };

        #[cfg(feature = "video")]
        let events = {
            let events = Box::pin(events) as Pin<Box<dyn Stream<Item = SocketOutput> + Send>>;

            if let Some(video) = self.video() {
                let video_events = WatchStream::new(video.frames())
                    .map(|frame| SocketOutput::VideoFrame { frame });

                Box::pin(select(events, video_events))
            } else {
                events
            }
        };

        events
    }

    async fn process_socket_input(&self, req: SocketInput) -> Result<()> {
        match req {
            SocketInput::Button { button, state } => {
                self.buttons()
                    .get(&button)
                    .ok_or("Unknown button")?
                    .set_state(state)?;
            }
            #[cfg(feature = "hid")]
            SocketInput::KeyboardKey { key, state } => {
                self.hid()
                    .and_then(|hid| hid.keyboard())
                    .ok_or("Keyboard disabled")?
                    .change_key(crate::hid::KeyStateChange::new(key, state))
                    .await?;
            }
            #[cfg(feature = "hid")]
            SocketInput::MouseButton { button, state } => {
                self.hid()
                    .and_then(|hid| hid.mouse())
                    .ok_or("Mouse disabled")?
                    .change_state(crate::hid::MouseStateChange::Button(
                        crate::hid::ButtonStateChange::new(button, state),
                    ))
                    .await?;
            }
            #[cfg(feature = "hid")]
            SocketInput::MousePointer { x, y } => {
                self.hid()
                    .and_then(|hid| hid.mouse())
                    .ok_or("Mouse disabled")?
                    .change_state(crate::hid::MouseStateChange::Pointer(
                        crate::hid::PointerValueChange::absolute((x, y)),
                    ))
                    .await?;
            }
            #[cfg(feature = "hid")]
            SocketInput::MouseWheel { wheel } => {
                self.hid()
                    .and_then(|hid| hid.mouse())
                    .ok_or("Mouse disabled")?
                    .change_state(crate::hid::MouseStateChange::Wheel(
                        crate::hid::WheelValueChange::absolute(wheel),
                    ))
                    .await?;
            }
        }

        Ok(())
    }
}
