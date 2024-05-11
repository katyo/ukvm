mod args;
mod buttons;
mod leds;
mod result;
mod server;

#[cfg(feature = "http")]
mod http;

#[cfg(feature = "dbus")]
mod dbus;

#[cfg(feature = "hid")]
mod hid;

#[cfg(feature = "video")]
mod video;

pub use tracing as log;

pub use args::Args;
pub use buttons::{Buttons, ButtonsConfig};
pub use leds::{Leds, LedsConfig};
pub use server::{GracefulShutdown, Server, ServerConfig, ServerRef};
pub use ukvm_core::{ButtonId, LedId};

#[cfg(feature = "http")]
pub use ukvm_core::{SocketInput, SocketOutput};

#[cfg(any(feature = "dbus", feature = "http"))]
pub use ukvm_core::BindAddr;

#[cfg(feature = "dbus")]
pub use ukvm_core::DBusAddr;

#[cfg(feature = "http")]
pub use ukvm_core::{HttpAddr, HttpBindAddr};

#[cfg(feature = "hid")]
pub use hid::{Hid, HidConfig};

#[cfg(feature = "video")]
pub use video::{Video, VideoConfig};

pub use result::{Error, Result};
