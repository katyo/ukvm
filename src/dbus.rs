use crate::addr::socket_addr_parse;
use core::str::FromStr;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf};

/// DBus service binding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "addr", rename_all = "lowercase")]
pub enum DBusAddr {
    /// System bus
    System,

    /// Session bus
    Session,

    /// TCP socket address
    #[serde(rename = "tcp")]
    Addr(SocketAddr),

    #[serde(rename = "unix")]
    /// Unix socket path
    Path(PathBuf),
}

impl FromStr for DBusAddr {
    type Err = String;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        let (proto, res) = uri
            .split_once("://")
            .ok_or_else(|| format!("The '<protocol>://<resource>' expected but given '{uri}'"))?;

        match proto {
            "unix" => Ok(Self::Path(res.into())),
            "tcp" => socket_addr_parse(res, 6667).map(Self::Addr),
            "" => match res {
                "system" => Ok(Self::System),
                "session" | "user" => Ok(Self::Session),
                _ => socket_addr_parse(res, 6667).map(Self::Addr),
            },
            _ => Err(format!("Unknown DBus protocol: {proto}"))?,
        }
    }
}
