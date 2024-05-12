use core::str::FromStr;
#[cfg(feature = "tracing-subscriber")]
use tracing_subscriber::EnvFilter;

#[cfg(any(feature = "http", feature = "dbus"))]
use crate::Addr;

/// Micro KVM client
#[derive(Debug, argp::FromArgs)]
pub struct Args {
    /// Show version and exit
    #[argp(switch, short = 'V')]
    pub version: bool,

    #[cfg(any(feature = "http", feature = "dbus"))]
    /// Service uri
    #[argp(option, from_str_fn(FromStr::from_str))]
    #[cfg_attr(feature = "dbus", argp(default = "\"dbus://system\".parse().unwrap()"))]
    #[cfg_attr(
        all(not(feature = "dbus"), feature = "http"),
        argp(default = "\"http://127.0.0.1\".parse().unwrap()")
    )]
    pub uri: Addr,

    /// Logging filter
    #[cfg(feature = "tracing-subscriber")]
    #[argp(option, short = 'l', from_str_fn(Args::parse_env_filter))]
    pub log: Option<EnvFilter>,

    /// Client action to do
    #[argp(subcommand)]
    pub action: Action,
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
}

#[derive(Debug, argp::FromArgs)]
#[argp(subcommand)]
pub enum Action {
    /// Show status
    Status(StatusArgs),

    /// Push buttons
    Button(ButtonArgs),
}

/// Show status
#[derive(Debug, argp::FromArgs)]
#[argp(subcommand, name = "status")]
pub struct StatusArgs {
}

/// Push buttons
#[derive(Debug, argp::FromArgs)]
#[argp(subcommand, name = "button")]
pub struct ButtonArgs {
    /// Press button
    #[argp(switch, short = 'p')]
    pub press: bool,
    /// Release button
    #[argp(switch, short = 'r')]
    pub release: bool,
    /// Specify delay (milliseconds)
    #[argp(option, short = 'd', default = "100")]
    pub delay: u32,
    #[argp(positional, default = "\"power\".into()")]
    pub button: String,
}
