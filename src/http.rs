use crate::{ButtonId, Error, LedId, Result, Server};
use futures::stream::select_all;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{
    fs::{metadata, remove_file},
    net::UnixListener,
    spawn,
    sync::Semaphore,
};
use tokio_stream::{
    wrappers::{UnixListenerStream, WatchStream},
    StreamExt,
};

/// HTTP service binding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "bind", rename_all = "lowercase")]
pub enum HttpBind {
    /// Network socket
    #[serde(rename = "tcp")]
    Addr(SocketAddr),

    #[serde(rename = "unix")]
    /// Unix socket
    Path(PathBuf),
}

impl warp::reject::Reject for Error {}

/// Server capabilities
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Capabilities {
    /// Present buttons
    pub buttons: Vec<ButtonId>,

    /// Present LEDs
    pub leds: Vec<LedId>,
}

impl From<&Server> for Capabilities {
    fn from(server: &Server) -> Self {
        Self {
            leds: server.leds().keys().copied().collect(),
            buttons: server.buttons().keys().copied().collect(),
        }
    }
}

/*
/// Server events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$", rename_all = "snake_case")]
pub enum Event {
    /// Button state change
    ButtonState { id: ButtonId, state: bool },

    /// LED state change
    LedState { id: LedId, state: bool },
}
*/

impl Server {
    pub async fn spawn_http(&self, bind: &HttpBind, stop: &Arc<Semaphore>) -> Result<()> {
        use warp::Filter;

        let bind = bind.clone();
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

        let capabilities = warp::path("capabilities")
            .and(warp::get())
            .and(server.clone())
            .map(|server: Server| warp::reply::json(&Capabilities::from(&server)));

        let buttons = warp::path("buttons");

        let buttons_list = buttons
            .and(warp::path::end())
            .and(warp::get())
            .and(server.clone())
            .map(|server: Server| {
                warp::reply::json(&server.buttons().keys().copied().collect::<Vec<_>>())
            });

        let button = buttons.and(warp::path::param::<ButtonId>());

        let button_state = button.and(warp::path("state")).and(warp::path::end());

        let button_state_get = button_state.and(warp::post()).and(server.clone()).and_then(
            |id: ButtonId, server: Server| async move {
                let state = server
                    .buttons()
                    .get(&id)
                    .ok_or_else(warp::reject::not_found)?
                    .state();
                Ok::<_, warp::Rejection>(warp::reply::json(&state))
            },
        );

        let button_state_set = button_state
            .and(warp::put())
            .and(warp::body::json())
            .and(server.clone())
            .and_then(|id: ButtonId, state: bool, server: Server| async move {
                server
                    .buttons()
                    .get(&id)
                    .ok_or_else(warp::reject::not_found)?
                    .set_state(state)?;
                Ok::<_, warp::Rejection>(warp::reply::json(&state))
            });

        let buttons_serve = buttons_list.or(button_state_get).or(button_state_set);

        let leds = warp::path("leds");

        let leds_list = leds
            .and(warp::path::end())
            .and(warp::get())
            .and(server.clone())
            .map(|server: Server| {
                warp::reply::json(&server.leds().keys().copied().collect::<Vec<_>>())
            });

        let led = buttons.and(warp::path::param::<LedId>());

        let led_state = led.and(warp::path("state")).and(warp::path::end());

        let led_state_get = led_state.and(warp::post()).and(server.clone()).and_then(
            |id: LedId, server: Server| async move {
                let state = server
                    .leds()
                    .get(&id)
                    .ok_or_else(warp::reject::not_found)?
                    .state();
                Ok::<_, warp::Rejection>(warp::reply::json(&state))
            },
        );

        let leds_serve = leds_list.or(led_state_get);

        let events = warp::path("events")
            .and(warp::path::end())
            .and(warp::get())
            .and(server.clone())
            .and_then(|server: Server| async move {
                let button_events = select_all(server.buttons().iter().map(|(id, button)| {
                    let id = *id;
                    WatchStream::new(button.watch()).map(move |state| {
                        Ok::<_, warp::Error>(
                            warp::sse::Event::default()
                                .event(if state {
                                    "button-press"
                                } else {
                                    "button-release"
                                })
                                .data(id.to_string()),
                        )
                    })
                }));

                let led_events = select_all(server.leds().iter().map(|(id, led)| {
                    let id = *id;
                    WatchStream::new(led.watch()).map(move |state| {
                        Ok::<_, warp::Error>(
                            warp::sse::Event::default()
                                .event(if state { "led-on" } else { "led-off" })
                                .data(id.to_string()),
                        )
                    })
                }));

                let events = button_events.merge(led_events);

                Ok::<_, warp::Rejection>(warp::sse::reply(warp::sse::keep_alive().stream(events)))
            });

        let http_server = warp::serve(
            index
                .or(capabilities)
                .or(events)
                .or(buttons_serve)
                .or(leds_serve),
        );

        match bind {
            HttpBind::Addr(addr) => {
                log::debug!("Starting {}", addr);

                let (addr, future) = http_server.bind_with_graceful_shutdown(addr, async move {
                    log::debug!("Await signal to stop");
                    let lock = stop.acquire().await;
                    log::debug!("Received stop signal");
                    drop(lock);
                });

                spawn(async move {
                    log::info!("Started {}", addr);
                    future.await;
                    log::info!("Stopped {}", addr);
                });
            }
            HttpBind::Path(path) => {
                use std::os::unix::fs::FileTypeExt;

                log::debug!("Starting {}", path.display());

                // Remove socket file if exists
                if let Ok(meta) = metadata(&path).await {
                    if meta.file_type().is_socket() {
                        remove_file(&path).await?;
                    }
                }

                let future = http_server.serve_incoming_with_graceful_shutdown(
                    UnixListenerStream::new(UnixListener::bind(&path)?),
                    async move {
                        log::debug!("Await signal to stop");
                        let lock = stop.acquire().await;
                        log::debug!("Stopped");
                        drop(lock);
                    },
                );

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
}
