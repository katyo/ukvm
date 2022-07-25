use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};

#[derive(Debug, clap::Parser)]
#[clap(author, version, about)]
pub struct Args {
    #[cfg(feature = "dbus")]
    /// Run DBus system service
    #[clap(short, long)]
    pub dbus: bool,

    /// HTTP host to bind to
    #[clap(short, long, value_parser, default_value_t = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)))]
    pub tcp: IpAddr,

    /// HTTP port to bind to
    #[clap(short, long, value_parser, default_value_t = 8080)]
    pub port: u16,

    /// HTTP unix socket to bind to
    #[clap(short, long, value_parser)]
    pub unix: Option<PathBuf>,

    /// Config file path
    #[clap(short, long, value_parser, default_value = "/etc/ubc.toml")]
    pub config: PathBuf,
}
