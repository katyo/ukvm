/// Unified result type
pub type Result<T> = std::result::Result<T, Error>;

macro_rules! error_impl {
    (
        $(#[$($meta:meta)*])*
            $type:ident {
                $(
                    $(#[$($variant_meta:meta)*])*
                        $variant_name:ident ( $variant_type:ty ) { $($variant_impl:ident)* },
                )+
            }
    ) => {
        $(#[$($meta)*])*
        pub enum $type {
            $(
                $(#[$($variant_meta)*])*
                $variant_name($variant_type),
            )+
        }

        impl std::error::Error for $type {}

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    $(
                        $(#[$($variant_meta)*])*
                        Self::$variant_name(error) =>
                            write!(f, concat!(stringify!($variant_name), "Error: {}"), error),
                    )*
                }
            }
        }

        impl AsRef<str> for $type {
            fn as_ref(&self) -> &str {
                match self {
                    $(
                        $(#[$($variant_meta)*])*
                            Self::$variant_name(_) =>
                            concat!(stringify!($variant_name), "Error"),
                    )*
                }
            }
        }

        $(
            error_impl!(@ $($variant_impl)* ( $type ) ( $(#[$($variant_meta)*])* ) $variant_name ( $variant_type ));
        )*
    };

    (@ ( $type:ident ) ( $(#[$($variant_meta:meta)*])* ) $variant_name:ident ( $variant_type:ty ) ) => {};

    (@ From ( $type:ident ) ( $(#[$($variant_meta:meta)*])* ) $variant_name:ident ( $variant_type:ty ) ) => {
        $(#[$($variant_meta)*])*
        impl From<$variant_type> for $type {
            fn from(error: $variant_type) -> Self {
                Self::$variant_name(error)
            }
        }
    };
}

error_impl! {
    /// Unified error type
    #[derive(Debug)]
    Error {
        Io(std::io::Error) { From },
        Addr(std::net::AddrParseError) { From },
        Num(std::num::ParseIntError) { From },
        Json(serde_json::Error) { From },
        Toml(toml::de::Error) { From },
        #[cfg(feature = "zbus")]
        DBus(zbus::Error) { From },
        Other(String) { From },
    }
}

impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Self::Other(error.into())
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(error: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::Other(error.to_string())
    }
}
