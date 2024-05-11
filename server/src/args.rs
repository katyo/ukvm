use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};
#[cfg(feature = "tracing-subscriber")]
use tracing_subscriber::EnvFilter;

#[cfg(feature = "http")]
use crate::{HttpBind, HttpBindWay};

#[cfg(feature = "dbus")]
use crate::DBusBind;

/// Micro KVM server
#[derive(Debug, argp::FromArgs)]
pub struct Args {
    /// Run server
    #[argp(switch, short = 'r')]
    pub run: bool,

    #[cfg(any(feature = "http", feature = "dbus"))]
    /// Service bindings
    #[argp(option, from_str_fn(Args::parse_bind))]
    pub bind: Vec<Bind>,

    /// Config file path
    #[argp(option, short = 'c', arg_name = "path", default = "\"/etc/ukvm.toml\".into()")]
    pub config: PathBuf,

    /// Logging filter
    #[cfg(feature = "tracing-subscriber")]
    #[argp(option, short = 'l', from_str_fn(Args::parse_env_filter))]
    pub log: Option<EnvFilter>,

    /// Logging to journald instead of stderr
    #[cfg(feature = "tracing-subscriber")]
    #[argp(switch, short = 'j')]
    pub journal: bool,
}

impl Args {
    /// Create args from command-line
    pub fn new() -> Self {
        argp::parse_args_or_exit(argp::DEFAULT)
    }

    #[cfg(feature = "tracing-subscriber")]
    fn parse_env_filter(val: &str) -> core::result::Result<EnvFilter, String> {
        Ok(EnvFilter::new(val))
    }

    fn parse_bind(val: &str) -> core::result::Result<Bind, String> {
        Bind::from_str(val).map_err(|err| err.to_string())
    }
}

#[cfg(any(feature = "http", feature = "dbus"))]
/// Service binding
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "proto", rename_all = "lowercase")]
pub enum Bind {
    #[cfg(feature = "http")]
    /// HTTP server
    ///
    /// http://addr[:port]
    /// http+unix://path
    Http(HttpBind),

    #[cfg(feature = "dbus")]
    /// DBus service
    ///
    /// dbus://system
    /// dbus+tcp://addr[:port]
    /// dbus+unix://path
    DBus(DBusBind),
}

impl FromStr for Bind {
    type Err = Error;

    fn from_str(uri: &str) -> Result<Self> {
        let (proto, bind) = uri
            .split_once("://")
            .ok_or_else(|| "The <protocol>://<resource> expected")?;

        if let Some((proto, sub_proto)) = proto.split_once('+') {
            #[cfg(feature = "http")]
            if proto == "http" {
                return if sub_proto == "unix" {
                    Ok(Bind::Http(HttpBind {
                        bind: HttpBindWay::Path(bind.into()),
                        ..Default::default()
                    }))
                } else {
                    http_addr(bind)
                };
            }

            #[cfg(feature = "dbus")]
            if proto == "dbus" {
                return if sub_proto == "unix" {
                    Ok(Bind::DBus(DBusBind::Path(bind.into())))
                } else if sub_proto == "tcp" {
                    dbus_addr(bind)
                } else {
                    dbus_bus(bind)
                };
            }
        } else {
            #[cfg(feature = "http")]
            if proto == "http" {
                return http_addr(bind);
            }

            #[cfg(feature = "dbus")]
            if proto == "dbus" {
                return dbus_bus(bind);
            }
        }

        Err(Error::from(format!("Invalid binding URI: {}", uri)))
    }
}

fn socket_addr(bind: &str, default_port: u16) -> Result<SocketAddr> {
    let (addr, port) = if let Some((addr, port)) = bind.split_once(':') {
        (addr, port.parse()?)
    } else {
        (bind, default_port)
    };

    let addr = addr.parse::<IpAddr>()?;

    Ok((addr, port).into())
}

#[cfg(feature = "http")]
fn http_addr(bind: &str) -> Result<Bind> {
    socket_addr(bind, 8080).map(|addr| {
        Bind::Http(HttpBind {
            bind: HttpBindWay::Addr(addr),
            ..Default::default()
        })
    })
}

#[cfg(feature = "dbus")]
fn dbus_addr(bind: &str) -> Result<Bind> {
    socket_addr(bind, 6667).map(|addr| Bind::DBus(DBusBind::Addr(addr)))
}

#[cfg(feature = "dbus")]
fn dbus_bus(bind: &str) -> Result<Bind> {
    Ok(Bind::DBus(match bind {
        "system" => DBusBind::System,
        "session" => DBusBind::Session,
        _ => return Err(Error::from(format!("Unknown DBus bus: {}", bind))),
    }))
}
