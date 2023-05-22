use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
#[cfg(feature = "zbus")]
use zbus::zvariant::{OwnedValue, Type, Value};

/// Button type
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    FromStr,
    Display,
)]
#[cfg_attr(feature = "zbus", derive(Type, Value, OwnedValue))]
#[cfg_attr(feature = "zbus", zvariant(signature = "s"))]
#[serde(rename_all = "kebab-case")]
#[display(style = "kebab-case")]
pub enum ButtonId {
    /// System power button
    Power = 1,

    /// System reset button
    Reset = 2,

    /// Clear CMOS button
    Clear = 3,
}
