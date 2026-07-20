use std::fmt;

use zeroize::Zeroizing;

macro_rules! redacted_diagnostic {
    ($name:ident, $label:literal) => {
        pub struct $name(Zeroizing<String>);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(Zeroizing::new(value.into()))
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                let _ = &self.0;
                formatter.write_str(concat!($label, "([REDACTED])"))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                let _ = &self.0;
                formatter.write_str("[REDACTED]")
            }
        }
    };
}

redacted_diagnostic!(RedactedCredential, "RedactedCredential");
redacted_diagnostic!(RedactedAddress, "RedactedAddress");
redacted_diagnostic!(RedactedSignature, "RedactedSignature");

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RedactedEvent {
    InternalBoundary { correlation_id: u64 },
}

impl RedactedEvent {
    #[must_use]
    pub const fn internal_boundary(correlation_id: u64) -> Self {
        Self::InternalBoundary { correlation_id }
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn from_sensitive_fixture() -> Self {
        Self::internal_boundary(1)
    }
}

impl fmt::Debug for RedactedEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for RedactedEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InternalBoundary { correlation_id } => write!(
                formatter,
                "event=credential_boundary category=internal_redacted correlation_id={correlation_id}"
            ),
        }
    }
}
