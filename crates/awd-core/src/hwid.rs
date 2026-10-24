//! Plug and Play hardware identifiers.

use std::fmt;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// A Plug and Play hardware or compatible identifier, for example
/// `PCI\VEN_10DE&DEV_2684&SUBSYS_167F10DE&REV_A1` or `USB\VID_046D&PID_C52B&MI_00`.
///
/// Windows compares these case-insensitively, and so does this type.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HardwareId(String);

impl HardwareId {
    /// Wraps an identifier, trimming surrounding whitespace.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into().trim().to_owned())
    }

    /// The identifier exactly as reported by the system.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Enumerator prefix (`PCI`, `USB`, `ACPI`, `HID`, `SWD`, ...), if present.
    pub fn enumerator(&self) -> Option<&str> {
        self.0
            .split_once('\\')
            .map(|(e, _)| e)
            .filter(|e| !e.is_empty())
    }

    fn body(&self) -> &str {
        self.0.split_once('\\').map_or(&self.0, |(_, b)| b)
    }

    /// Value of a `KEY_VALUE` field inside the id, e.g. `field("VEN")` on a PCI id.
    pub fn field(&self, key: &str) -> Option<&str> {
        self.body().split('&').find_map(|part| {
            let (k, v) = part.split_once('_')?;
            k.eq_ignore_ascii_case(key).then_some(v)
        })
    }

    /// 16-bit vendor id: `VEN_` for PCI, `VID_` for USB/HID.
    pub fn vendor_id(&self) -> Option<u16> {
        self.field("VEN")
            .or_else(|| self.field("VID"))
            .and_then(parse_hex16)
    }

    /// 16-bit product/device id: `DEV_` for PCI, `PID_` for USB/HID.
    pub fn product_id(&self) -> Option<u16> {
        self.field("DEV")
            .or_else(|| self.field("PID"))
            .and_then(parse_hex16)
    }

    /// `true` for class-level ids such as `PCI\CC_0300` or `USB\Class_FF` that
    /// match a whole family of devices rather than one product.
    pub fn is_generic(&self) -> bool {
        self.vendor_id().is_none() && self.product_id().is_none()
    }

    /// Rough specificity: the number of `&`-separated fields. Windows lists ids
    /// from most to least specific; this lets callers restore that order.
    pub fn specificity(&self) -> usize {
        self.body().split('&').filter(|p| !p.is_empty()).count()
    }
}

fn parse_hex16(s: &str) -> Option<u16> {
    u16::from_str_radix(s.get(..4)?, 16).ok()
}

impl PartialEq for HardwareId {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for HardwareId {}

impl Hash for HardwareId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for b in self.0.bytes() {
            state.write_u8(b.to_ascii_uppercase());
        }
    }
}

impl fmt::Display for HardwareId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for HardwareId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for HardwareId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pci_fields() {
        let id = HardwareId::new(r"PCI\VEN_10DE&DEV_2684&SUBSYS_167F10DE&REV_A1");
        assert_eq!(id.enumerator(), Some("PCI"));
        assert_eq!(id.vendor_id(), Some(0x10DE));
        assert_eq!(id.product_id(), Some(0x2684));
        assert_eq!(id.field("SUBSYS"), Some("167F10DE"));
        assert_eq!(id.field("rev"), Some("A1"));
        assert_eq!(id.specificity(), 4);
        assert!(!id.is_generic());
    }

    #[test]
    fn parses_usb_fields() {
        let id = HardwareId::new(r"USB\VID_046D&PID_C52B&MI_01");
        assert_eq!(id.vendor_id(), Some(0x046D));
        assert_eq!(id.product_id(), Some(0xC52B));
        assert_eq!(id.field("MI"), Some("01"));
    }

    #[test]
    fn class_ids_are_generic() {
        assert!(HardwareId::new(r"PCI\CC_0300").is_generic());
        assert!(HardwareId::new(r"USB\Class_FF&SubClass_01").is_generic());
        assert!(!HardwareId::new(r"PCI\VEN_8086&CC_0300").is_generic());
    }

    #[test]
    fn comparison_is_case_insensitive() {
        let a = HardwareId::new(r"usb\vid_046d&pid_c52b");
        let b = HardwareId::new(r"USB\VID_046D&PID_C52B");
        assert_eq!(a, b);
        let mut set = std::collections::HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn tolerates_ids_without_enumerator() {
        let id = HardwareId::new("*PNP0C0A");
        assert_eq!(id.enumerator(), None);
        assert_eq!(id.vendor_id(), None);
        assert_eq!(id.specificity(), 1);
    }

    #[test]
    fn serializes_as_plain_string() {
        let id = HardwareId::new(r"ACPI\VEN_INT&DEV_33A0");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""ACPI\\VEN_INT&DEV_33A0""#);
        let back: HardwareId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }
}
