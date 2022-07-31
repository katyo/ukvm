use crate::{ButtonId, Error, LedId, Result, Server, ServerEvent};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{
    fs::{metadata, remove_file},
    net::UnixListener,
    spawn,
    sync::Semaphore,
};
use tokio_stream::{wrappers::UnixListenerStream, StreamExt};

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

impl Server {
    pub async fn spawn_http(&self, bind: &HttpBind, stop: &Arc<Semaphore>) -> Result<()> {
        use warp::Filter;

        let bind = bind.clone();
        let stop = stop.clone();

        let server = warp::any().map({
            let server = self.clone();
            move || server.clone()
        });

        #[cfg(not(feature = "web"))]
        let index = warp::path::end().map(|| {
            warp::http::Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .body(include_str!("index.html"))
        });

        #[cfg(feature = "web")]
        let index = include!(concat!(env!("OUT_DIR"), "/web.rs"));

        let capabilities = warp::path!("capabilities")
            .and(server.clone())
            .map(|server: Server| warp::reply::json(&server.capabilities()));

        let buttons = warp::path("buttons");
        let button = buttons.and(warp::path::param::<ButtonId>());

        let buttons_list = buttons
            .and(warp::path::end())
            .and(server.clone())
            .map(|server: Server| warp::reply::json(&server.buttons().collect::<Vec<_>>()));

        let button_press = button
            .and(warp::path("press"))
            .and(warp::path::end())
            .and(server.clone())
            .and_then(|id: ButtonId, server: Server| async move {
                server.button_press(id).await?;
                Ok::<_, warp::Rejection>(warp::reply::json(&true))
            });

        let leds = warp::path("leds");
        let led = buttons.and(warp::path::param::<LedId>());

        let leds_list = leds
            .and(warp::path::end())
            .and(server.clone())
            .map(|server: Server| warp::reply::json(&server.leds().collect::<Vec<_>>()));

        let led_status = led
            .and(warp::path("status"))
            .and(warp::path::end())
            .and(server.clone())
            .and_then(|id: LedId, server: Server| async move {
                let status = server.led_status(id)?;
                Ok::<_, warp::Rejection>(warp::reply::json(&status))
            });

        let events = warp::path("events")
            .and(warp::path::end())
            .and(server.clone())
            .and_then(|server: Server| async move {
                let events = server.events().await?;

                Ok::<_, warp::Rejection>(warp::sse::reply(warp::sse::keep_alive().stream(
                    events.map(|event| {
                        Ok::<_, warp::Error>(match event {
                            ServerEvent::LedStatus { id, status } => warp::sse::Event::default()
                                .event(if status { "led-on" } else { "led-off" })
                                .data(id.to_string()),
                            ServerEvent::ButtonPress { id } => warp::sse::Event::default()
                                .event("button-press")
                                .data(id.to_string()),
                        })
                    }),
                )))
            });

        let http_server = warp::serve(
            warp::get()
                .and(
                    index
                        .or(capabilities)
                        .or(events)
                        .or(buttons_list)
                        .or(leds_list),
                )
                .or(warp::post().and(button_press.or(led_status))),
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
                        let _ = stop.acquire().await;
                        log::debug!("Stopped");
                        let _ = stop.acquire().await;
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
