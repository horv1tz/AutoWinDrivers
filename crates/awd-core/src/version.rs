//! Driver version numbers (`major.minor.build.revision`).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A four-part driver version as found in `DriverVer` INF directives and the
/// `DEVPKEY_Device_DriverVersion` property. Compares numerically, part by part.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DriverVersion([u32; 4]);

impl DriverVersion {
    /// Builds a version from its four parts.
    pub const fn new(major: u32, minor: u32, build: u32, revision: u32) -> Self {
        Self([major, minor, build, revision])
    }

    /// The four numeric parts, missing ones filled with zero.
    pub const fn parts(&self) -> [u32; 4] {
        self.0
    }
}

/// Error returned when a version string cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid driver version {0:?}")]
pub struct ParseVersionError(pub String);

impl FromStr for DriverVersion {
    type Err = ParseVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let err = || ParseVersionError(s.to_owned());
        if s.is_empty() {
            return Err(err());
        }
        let mut parts = [0u32; 4];
        for (i, piece) in s.split('.').enumerate() {
            let slot = parts.get_mut(i).ok_or_else(err)?;
            *slot = piece.trim().parse().map_err(|_| err())?;
        }
        Ok(Self(parts))
    }
}

impl fmt::Display for DriverVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d] = self.0;
        write!(f, "{a}.{b}.{c}.{d}")
    }
}

impl Serialize for DriverVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for DriverVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_and_short_versions() {
        assert_eq!(
            "31.0.15.3623".parse::<DriverVersion>().unwrap(),
            DriverVersion::new(31, 0, 15, 3623)
        );
        assert_eq!(
            "10.0".parse::<DriverVersion>().unwrap(),
            DriverVersion::new(10, 0, 0, 0)
        );
        assert_eq!(
            "7".parse::<DriverVersion>().unwrap(),
            DriverVersion::new(7, 0, 0, 0)
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!("".parse::<DriverVersion>().is_err());
        assert!("1.2.3.4.5".parse::<DriverVersion>().is_err());
        assert!("1.x".parse::<DriverVersion>().is_err());
        assert!("v1.0".parse::<DriverVersion>().is_err());
    }

    #[test]
    fn compares_numerically_not_lexically() {
        let old: DriverVersion = "9.18.13.4052".parse().unwrap();
        let new: DriverVersion = "10.18.13.5362".parse().unwrap();
        assert!(new > old);
        let a: DriverVersion = "31.0.15.3623".parse().unwrap();
        let b: DriverVersion = "31.0.15.3667".parse().unwrap();
        assert!(b > a);
    }

    #[test]
    fn serde_round_trip_uses_string_form() {
        let v = DriverVersion::new(32, 0, 15, 6094);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#""32.0.15.6094""#);
        let back: DriverVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }
}
