use ukvm::{Args, GracefulShutdown, Result, Server, ServerConfig};

pub use tracing as log;

use tokio::{
    select,
    signal::unix::{signal, SignalKind},
};

#[cfg_attr(not(feature = "multi-thread"), tokio::main(flavor = "current_thread"))]
#[cfg_attr(feature = "multi-thread", tokio::main)]
async fn main() -> Result<()> {
    let args = Args::from_cmdline();

    if args.version {
        println!(
            "{name} {version}",
            name = env!("CARGO_PKG_NAME"),
            version = env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }

    #[cfg(feature = "tracing-subscriber")]
    if let Some(log) = args.log {
        use tracing_subscriber::prelude::*;

        let registry = tracing_subscriber::registry().with(log);

        #[cfg(all(feature = "stderr", feature = "journal"))]
        let registry = registry.with(if !args.journal {
            Some(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr))
        } else {
            None
        });

        #[cfg(all(feature = "stderr", not(feature = "journal")))]
        let registry =
            registry.with(tracing_subscriber::fmt::Layer::default().with_writer(std::io::stderr));

        #[cfg(feature = "journal")]
        let registry = registry.with(if args.journal {
            Some(tracing_journald::Layer::new()?)
        } else {
            None
        });

        registry.init();
    }

    let mut run = true;

    let mut intr = signal(SignalKind::interrupt())?;
    let mut term = signal(SignalKind::terminate())?;
    let mut usr1 = signal(SignalKind::user_defined1())?;

    while run {
        let config = ServerConfig::from_file(&args.config).await?;

        log::debug!("Config: {:#?}", config);

        if !args.run {
            break;
        }

        let gs = GracefulShutdown::default();

        // create server instance
        let server = Server::new(&config).await?;

        log::info!("Starting");

        // start server interfaces
        server
            .spawn(args.bind.iter().chain(&config.binds), &gs)
            .await?;

        log::info!("Started");

        select! {
            // stop server
            _ = intr.recv() => {
                log::info!("Interrupt");
                run = false;
            }
            // stop server
            _ = term.recv() => {
                log::info!("Terminate");
                run = false;
            }
            // reload server
            _ = usr1.recv() => {
                log::info!("Reload");
            }
        }

        // stop server interfaces
        gs.shutdown().await;
    }

    log::info!("Bye");

    Ok(())
}
