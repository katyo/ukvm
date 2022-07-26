use crate::{ButtonType, Result, Server};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{net::UnixListener, spawn, sync::Semaphore};
use tokio_stream::wrappers::UnixListenerStream;

/// HTTP service binding
#[derive(Debug, Clone)]
pub enum HttpBind {
    /// Network socket
    Addr(SocketAddr),

    /// Unix socket
    Path(PathBuf),
}

#[derive(Debug)]
struct AnyhowReject(anyhow::Error);

impl warp::reject::Reject for AnyhowReject {}

pub(crate) fn anyhow_reject(error: impl Into<anyhow::Error>) -> warp::Rejection {
    warp::reject::custom(AnyhowReject(error.into()))
}

const INDEX_HTML: &'static str = include_str!("index.html");

impl Server {
    pub async fn spawn_http(&self, bind: &HttpBind, stop: &Arc<Semaphore>) -> Result<()> {
        use warp::Filter;

        let bind = bind.clone();
        let stop = stop.clone();

        let server = warp::any().map({
            let server = self.clone();
            move || server.clone()
        });

        let get_root = warp::path::end().map(|| {
            warp::http::Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .body(INDEX_HTML)
        });

        let get_capabilities = warp::path!("capabilities")
            .and(server.clone())
            .map(|server: Server| warp::reply::json(&server.capabilities()));

        let post_button_press = warp::path("buttons")
            .and(warp::path::param())
            .and(server.clone())
            .and_then(|button: ButtonType, server: Server| async move {
                server
                    .button_press(button)
                    .await
                    .map(|_| "")
                    .map_err(anyhow_reject)
            });

        let http_server = warp::serve(
            warp::get()
                .and(get_root.or(get_capabilities))
                .or(warp::post().and(post_button_press)),
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
                log::debug!("Starting {}", path.display());

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
                });
            }
        }

        Ok(())
    }
}
