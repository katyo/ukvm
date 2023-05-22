use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
#[cfg(feature = "zbus")]
use zbus::zvariant::{OwnedValue, Type, Value};

/// LED type
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Deserialize,
    Serialize,
    FromStr,
    Display,
)]
#[cfg_attr(feature = "zbus", derive(Type, Value, OwnedValue))]
#[cfg_attr(feature = "zbus", zvariant(signature = "s"))]
#[serde(rename_all = "kebab-case")]
#[display(style = "kebab-case")]
pub enum LedId {
    /// Power status LED
    Power = 1,

    /// Disk usage LED
    Disk = 2,

    /// Ethernet usage LED
    Ether = 3,
}
