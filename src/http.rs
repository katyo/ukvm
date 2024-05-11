use crate::{
    addr::socket_addr_parse,
    hid::{Button, Key, KeyboardState, Led, MouseState},
    ButtonId, LedId,
};
use core::str::FromStr;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

#[cfg(feature = "video")]
use std::sync::Arc;

/// HTTP service options
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpBindAddr {
    /// Bind way
    #[serde(flatten)]
    pub addr: HttpAddr,

    #[cfg(feature = "tls")]
    #[serde(flatten)]
    /// Enable TLS encryption
    pub tls: Option<HttpTlsOpts>,
}

impl FromStr for HttpBindAddr {
    type Err = String;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            addr: uri.parse()?,
            ..Default::default()
        })
    }
}

/// HTTP service options
#[cfg(feature = "tls")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HttpTlsOpts {
    #[cfg(feature = "tls")]
    /// Path to private key file
    pub key: PathBuf,

    #[cfg(feature = "tls")]
    /// Path to certificate file
    pub cert: PathBuf,

    #[cfg(feature = "tls")]
    /// Path to client auth file
    pub auth: Option<PathBuf>,
}

#[cfg(feature = "tls")]
impl Default for HttpTlsOpts {
    fn default() -> Self {
        Self {
            key: PathBuf::from("/etc/ukvm/key.pem"),
            cert: PathBuf::from("/etc/ukvm/cert.pem"),
            auth: None,
        }
    }
}

/// HTTP service binding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "addr", rename_all = "lowercase")]
pub enum HttpAddr {
    /// Network socket
    #[serde(rename = "tcp")]
    Addr(SocketAddr),

    #[serde(rename = "unix")]
    /// Unix socket
    Path(PathBuf),
}

impl Default for HttpAddr {
    fn default() -> Self {
        Self::Addr(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            8080,
        ))
    }
}

impl FromStr for HttpAddr {
    type Err = String;

    fn from_str(uri: &str) -> Result<Self, Self::Err> {
        let (proto, res) = uri
            .split_once("://")
            .ok_or_else(|| format!("The '<protocol>://<resource>' expected but given '{uri}'"))?;

        match proto {
            "unix" => Ok(Self::Path(res.into())),
            "tcp" | "" => socket_addr_parse(res, 8080).map(Self::Addr),
            _ => Err(format!("Unknown HTTP protocol: {proto}"))?,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$")]
pub enum SocketInput {
    #[serde(rename = "b")]
    Button {
        #[serde(rename = "b")]
        button: ButtonId,
        #[serde(rename = "s")]
        state: bool,
    },
    #[cfg(feature = "hid")]
    #[serde(rename = "k")]
    KeyboardKey {
        #[serde(rename = "k")]
        key: Key,
        #[serde(rename = "s")]
        state: bool,
    },
    #[cfg(feature = "hid")]
    #[serde(rename = "m")]
    MouseButton {
        #[serde(rename = "b")]
        button: Button,
        #[serde(rename = "s")]
        state: bool,
    },
    #[cfg(feature = "hid")]
    #[serde(rename = "p")]
    MousePointer { x: i16, y: i16 },
    #[cfg(feature = "hid")]
    #[serde(rename = "p")]
    MouseWheel {
        #[serde(rename = "w")]
        wheel: i8,
    },
}

/// Outgoing message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$")]
pub enum SocketOutput {
    /// Initial state
    #[serde(rename = "s")]
    State {
        /// Led states
        #[serde(rename = "l")]
        leds: HashMap<LedId, bool>,
        /// Button states
        #[serde(rename = "b")]
        buttons: HashMap<ButtonId, bool>,
        /// Keyboard state
        #[cfg(feature = "hid")]
        #[serde(rename = "k")]
        keyboard: Option<KeyboardState>,
        /// Mouse state
        #[cfg(feature = "hid")]
        #[serde(rename = "m")]
        mouse: Option<MouseState>,
    },
    /// LED state change
    #[serde(rename = "l")]
    Led {
        #[serde(rename = "l")]
        led: LedId,
        #[serde(rename = "s")]
        state: bool,
    },
    /// Button state change
    #[serde(rename = "b")]
    Button {
        #[serde(rename = "b")]
        button: ButtonId,
        #[serde(rename = "s")]
        state: bool,
    },
    /// Keyboard key state change
    #[cfg(feature = "hid")]
    #[serde(rename = "k")]
    KeyboardKey {
        #[serde(rename = "k")]
        key: Key,
        #[serde(rename = "s")]
        state: bool,
    },
    /// Keyboard led state change
    #[cfg(feature = "hid")]
    #[serde(rename = "i")]
    KeyboardLed {
        #[serde(rename = "l")]
        led: Led,
        #[serde(rename = "s")]
        state: bool,
    },
    /// Mouse button state change
    #[cfg(feature = "hid")]
    #[serde(rename = "m")]
    MouseButton {
        #[serde(rename = "b")]
        button: Button,
        #[serde(rename = "s")]
        state: bool,
    },
    /// Mouse pointer change
    #[cfg(feature = "hid")]
    #[serde(rename = "p")]
    MousePointer { x: i16, y: i16 },
    /// Mouse wheel change
    #[cfg(feature = "hid")]
    #[serde(rename = "p")]
    MouseWheel {
        #[serde(rename = "w")]
        wheel: i8,
    },
    /// Video frames
    #[cfg(feature = "video")]
    #[serde(rename = "v")]
    VideoFrame {
        #[serde(rename = "f")]
        frame: Arc<Vec<u8>>,
    },
}
