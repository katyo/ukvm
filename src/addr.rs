use core::str::FromStr;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};

#[cfg(feature = "dbus")]
use crate::DBusAddr;

#[cfg(feature = "http")]
use crate::{HttpAddr, HttpBindAddr};

macro_rules! addr_impl {
    ($($atype:ident: $hatype:ty,)*) => {
        $(
            #[cfg(any(feature = "http", feature = "dbus"))]
            /// Unified address
            #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
            #[serde(tag = "proto", rename_all = "lowercase")]
            pub enum $atype {
                #[cfg(feature = "http")]
                /// HTTP server
                ///
                /// http://addr[:port]
                /// http+unix://path
                Http($hatype),

                #[cfg(feature = "dbus")]
                /// DBus service
                ///
                /// dbus://system
                /// dbus://addr[:port]
                /// dbus+unix://path
                DBus(DBusAddr),
            }

            impl FromStr for $atype {
                type Err = String;

                fn from_str(uri: &str) -> Result<Self, Self::Err> {
                    let (proto, offset) = if let Some((proto, _)) = uri.split_once("://") {
                        proto
                            .split_once('+')
                            .map(|(proto, _)| (proto, proto.len() + 1))
                            .unwrap_or((proto, proto.len()))
                    } else {
                        Err(format!("Invalid binding URI: {}", uri))?
                    };

                    let uri = &uri[offset..];

                    match proto {
                        #[cfg(feature = "http")]
                        "http" => <$hatype>::from_str(uri).map(Self::Http),
                        #[cfg(feature = "dbus")]
                        "dbus" => DBusAddr::from_str(uri).map(Self::DBus),
                        _ => Err(format!("Unknown protocol: {proto}"))?,
                    }
                }
            }
        )*
    }
}

addr_impl!{
    Addr: HttpAddr,
    BindAddr: HttpBindAddr,
}

#[cfg(any(feature = "dbus", feature = "http"))]
pub fn socket_addr_parse(bind: &str, default_port: u16) -> Result<SocketAddr, String> {
    let (addr, port) = if let Some((addr, port)) = bind.split_once(':') {
        (addr, port.parse::<u16>().map_err(|e| e.to_string())?)
    } else {
        (bind, default_port)
    };

    let addr = addr.parse::<IpAddr>().map_err(|e| e.to_string())?;

    Ok((addr, port).into())
}

#[cfg(test)]
mod test {
    use super::{Addr, BindAddr};

    #[cfg(feature = "dbus")]
    use super::DBusAddr;

    #[cfg(feature = "http")]
    use super::{HttpAddr, HttpBindAddr};

    #[cfg(feature = "dbus")]
    #[test]
    fn dbus_addr_from_str() {
        assert_eq!(
            "dbus://system".parse::<Addr>(),
            Ok(Addr::DBus(DBusAddr::System))
        );
        assert_eq!(
            "dbus://session".parse::<Addr>(),
            Ok(Addr::DBus(DBusAddr::Session))
        );
        assert_eq!(
            "dbus://user".parse::<Addr>(),
            Ok(Addr::DBus(DBusAddr::Session))
        );
        assert_eq!(
            "dbus://192.168.1.10:1818".parse::<Addr>(),
            Ok(Addr::DBus(DBusAddr::Addr(
                "192.168.1.10:1818".parse().unwrap()
            )))
        );
        assert_eq!(
            "dbus+unix:///run/bus".parse::<Addr>(),
            Ok(Addr::DBus(DBusAddr::Path("/run/bus".into())))
        );
    }

    #[cfg(feature = "http")]
    #[test]
    fn http_addr_from_str() {
        assert_eq!(
            "http://192.168.1.10:1818".parse::<Addr>(),
            Ok(Addr::Http(HttpAddr::Addr(
                "192.168.1.10:1818".parse().unwrap()
            )))
        );
        assert_eq!(
            "http+unix:///run/sock".parse::<Addr>(),
            Ok(Addr::Http(HttpAddr::Path("/run/sock".into())))
        );

        assert_eq!(
            "http://192.168.1.10:1818".parse::<BindAddr>(),
            Ok(BindAddr::Http(HttpBindAddr {
                addr: HttpAddr::Addr("192.168.1.10:1818".parse().unwrap()),
                ..Default::default()
            }))
        );
        assert_eq!(
            "http+unix:///run/sock".parse::<BindAddr>(),
            Ok(BindAddr::Http(HttpBindAddr {
                addr: HttpAddr::Path("/run/sock".into()),
                ..Default::default()
            }))
        );
    }
}
