//! A completed scan and its summary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::device::{Device, DeviceStatus};

/// Facts about the machine that was scanned.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostInfo {
    /// Computer name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    /// Operating system description, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    /// CPU architecture the scanner ran on (`x86_64`, `aarch64`).
    pub arch: String,
}

impl HostInfo {
    /// Host info for the current process without any platform calls.
    pub fn current() -> Self {
        Self {
            hostname: std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .ok()
                .filter(|s| !s.is_empty()),
            os: None,
            arch: std::env::consts::ARCH.to_owned(),
        }
    }
}

/// The result of one device scan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScanReport {
    /// When the scan finished.
    pub scanned_at: DateTime<Utc>,
    /// The scanned machine.
    pub host: HostInfo,
    /// All devices returned by the source.
    pub devices: Vec<Device>,
}

impl ScanReport {
    /// Wraps devices into a report timestamped now.
    pub fn new(host: HostInfo, devices: Vec<Device>) -> Self {
        Self {
            scanned_at: Utc::now(),
            host,
            devices,
        }
    }

    /// Counts devices by state.
    pub fn summary(&self) -> ScanSummary {
        ScanSummary::of(&self.devices)
    }
}

/// Device counts by state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanSummary {
    /// Every device in the report.
    pub total: usize,
    /// Present and working.
    pub ok: usize,
    /// Present with a problem (excluding disabled).
    pub problems: usize,
    /// Of the problems, those with no usable driver.
    pub missing_driver: usize,
    /// Present but disabled.
    pub disabled: usize,
    /// Not connected.
    pub not_present: usize,
    /// Status could not be read.
    pub unknown: usize,
}

impl ScanSummary {
    /// Counts the given devices.
    pub fn of(devices: &[Device]) -> Self {
        let mut s = Self {
            total: devices.len(),
            ..Self::default()
        };
        for d in devices {
            match d.status {
                DeviceStatus::Ok => s.ok += 1,
                DeviceStatus::Problem { code } => {
                    s.problems += 1;
                    if code.needs_driver() {
                        s.missing_driver += 1;
                    }
                }
                DeviceStatus::Disabled => s.disabled += 1,
                DeviceStatus::NotPresent => s.not_present += 1,
                DeviceStatus::Unknown => s.unknown += 1,
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../../../tests/fixtures/scan-sample.json");

    #[test]
    fn sample_fixture_parses_and_summarizes() {
        let report: ScanReport = serde_json::from_str(SAMPLE).expect("fixture parses");
        let s = report.summary();
        assert_eq!(s.total, 6);
        assert_eq!(s.ok, 3);
        assert_eq!(s.problems, 1);
        assert_eq!(s.missing_driver, 1);
        assert_eq!(s.disabled, 1);
        assert_eq!(s.not_present, 1);
        assert_eq!(s.unknown, 0);

        let gpu = report
            .devices
            .iter()
            .find(|d| d.name.contains("RTX"))
            .expect("gpu in fixture");
        assert_eq!(gpu.hardware_ids[0].vendor_id(), Some(0x10DE));
        let drv = gpu.driver.as_ref().unwrap();
        assert_eq!(drv.version.unwrap().parts(), [32, 0, 15, 6094]);
        assert!(!drv.is_inbox);
    }

    #[test]
    fn fixture_round_trips_through_serde() {
        let report: ScanReport = serde_json::from_str(SAMPLE).unwrap();
        let json = serde_json::to_string(&report).unwrap();
        let again: ScanReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, again);
    }
}
