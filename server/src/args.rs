#[cfg(any(feature = "dbus", feature = "http"))]
use crate::BindAddr;
use std::{path::PathBuf, str::FromStr};
#[cfg(feature = "tracing-subscriber")]
use tracing_subscriber::EnvFilter;

/// Micro KVM server
#[derive(Debug, argp::FromArgs)]
pub struct Args {
    /// Show version and exit
    #[argp(switch, short = 'V')]
    pub version: bool,

    /// Run server
    #[argp(switch, short = 'r')]
    pub run: bool,

    #[cfg(any(feature = "http", feature = "dbus"))]
    /// Service bindings
    #[argp(option, from_str_fn(FromStr::from_str))]
    pub bind: Vec<BindAddr>,

    /// Config file path
    #[argp(
        option,
        short = 'c',
        arg_name = "path",
        default = "\"/etc/ukvm.toml\".into()"
    )]
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
    pub fn from_cmdline() -> Self {
        argp::parse_args_or_exit(argp::DEFAULT)
    }

    #[cfg(feature = "tracing-subscriber")]
    fn parse_env_filter(val: &str) -> core::result::Result<EnvFilter, String> {
        Ok(EnvFilter::new(val))
    }
}
