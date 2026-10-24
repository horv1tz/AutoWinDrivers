//! The device model shared by every front end.

use std::fmt;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::hwid::HardwareId;
use crate::problem::ProblemCode;
use crate::version::DriverVersion;

/// A device instance as seen by the Plug and Play manager.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Device {
    /// Device instance path, e.g. `PCI\VEN_10DE&DEV_2684&SUBSYS_167F10DE&REV_A1\4&2283F625&0&0019`.
    pub instance_id: String,
    /// Best available display name: friendly name, else device description, else instance id.
    pub name: String,
    /// Device description as reported by the driver or bus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Setup class name (`Display`, `Net`, `USB`, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// Setup class GUID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_guid: Option<Uuid>,
    /// Manufacturer from the installed INF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    /// Enumerator that found the device (`PCI`, `USB`, `ACPI`, `SWD`, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enumerator: Option<String>,
    /// Hardware ids, most specific first.
    #[serde(default)]
    pub hardware_ids: Vec<HardwareId>,
    /// Compatible ids, most specific first.
    #[serde(default)]
    pub compatible_ids: Vec<HardwareId>,
    /// Human-readable location (`PCI bus 1, device 0, function 0`, `Port_#0003.Hub_#0001`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Instance id of the parent device node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Whether the device is currently connected.
    pub present: bool,
    /// Raw `DN_*` status flags from `CM_Get_DevNode_Status`, for diagnostics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub devnode_status: Option<u32>,
    /// Interpreted status.
    pub status: DeviceStatus,
    /// The currently installed driver, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver: Option<InstalledDriver>,
}

impl Device {
    /// The problem code, if the device has one (includes "disabled").
    pub fn problem_code(&self) -> Option<ProblemCode> {
        match self.status {
            DeviceStatus::Problem { code } => Some(code),
            DeviceStatus::Disabled => Some(ProblemCode::DISABLED),
            _ => None,
        }
    }

    /// `true` when the device is present and reports a problem other than being disabled.
    pub fn has_problem(&self) -> bool {
        matches!(self.status, DeviceStatus::Problem { .. })
    }

    /// `true` when the device is present and has no usable driver.
    pub fn needs_driver(&self) -> bool {
        match self.status {
            DeviceStatus::Problem { code } => code.needs_driver(),
            _ => false,
        }
    }

    /// All ids a driver could match, hardware ids first.
    pub fn all_ids(&self) -> impl Iterator<Item = &HardwareId> {
        self.hardware_ids.iter().chain(self.compatible_ids.iter())
    }
}

/// Interpreted device state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeviceStatus {
    /// Present, started, no problem.
    Ok,
    /// Present with a problem code other than "disabled".
    Problem {
        /// The Device Manager problem code.
        code: ProblemCode,
    },
    /// Present but disabled by the user or policy (code 22).
    Disabled,
    /// Known to the system but not currently connected.
    NotPresent,
    /// The status could not be read.
    Unknown,
}

impl DeviceStatus {
    /// Builds a status from what the Plug and Play manager reports.
    ///
    /// `problem` is the raw problem number (0 when there is none); `None`
    /// means the status could not be read at all.
    pub fn from_cm(present: bool, problem: Option<u32>) -> Self {
        if !present {
            return Self::NotPresent;
        }
        match problem {
            None => Self::Unknown,
            Some(0) => Self::Ok,
            Some(22) => Self::Disabled,
            Some(code) => Self::Problem {
                code: ProblemCode(code),
            },
        }
    }
}

impl fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ok => f.write_str("OK"),
            Self::Problem { code } => write!(f, "Problem {code}"),
            Self::Disabled => f.write_str("Disabled"),
            Self::NotPresent => f.write_str("Not present"),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}

/// The driver currently bound to a device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InstalledDriver {
    /// Driver provider (`NVIDIA`, `Intel`, `Microsoft`, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Driver description from the INF.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Parsed driver version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<DriverVersion>,
    /// Version string as reported, kept when it does not parse as four numbers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_raw: Option<String>,
    /// `DriverVer` date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<NaiveDate>,
    /// INF file name (`oem12.inf` for third-party, e.g. `usbport.inf` for inbox).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inf_path: Option<String>,
    /// INF install section used for this device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inf_section: Option<String>,
    /// The hardware or compatible id the INF matched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matching_device_id: Option<HardwareId>,
    /// Windows driver rank for the current match (lower is better).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    /// `true` when the INF ships with Windows rather than being an `oemNN.inf`.
    #[serde(default)]
    pub is_inbox: bool,
}

impl InstalledDriver {
    /// Derives `is_inbox` from the INF file name: third-party drivers are
    /// always staged as `oemNN.inf`.
    pub fn inf_is_inbox(inf_path: &str) -> bool {
        let file = inf_path.rsplit(['\\', '/']).next().unwrap_or(inf_path);
        !(file.len() > 3 && file[..3].eq_ignore_ascii_case("oem"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_from_cm_values() {
        assert_eq!(
            DeviceStatus::from_cm(false, Some(45)),
            DeviceStatus::NotPresent
        );
        assert_eq!(DeviceStatus::from_cm(true, Some(0)), DeviceStatus::Ok);
        assert_eq!(
            DeviceStatus::from_cm(true, Some(22)),
            DeviceStatus::Disabled
        );
        assert_eq!(DeviceStatus::from_cm(true, None), DeviceStatus::Unknown);
        assert_eq!(
            DeviceStatus::from_cm(true, Some(28)),
            DeviceStatus::Problem {
                code: ProblemCode::FAILED_INSTALL
            }
        );
    }

    #[test]
    fn inbox_detection_by_inf_name() {
        assert!(InstalledDriver::inf_is_inbox("usbport.inf"));
        assert!(InstalledDriver::inf_is_inbox(r"C:\Windows\INF\machine.inf"));
        assert!(!InstalledDriver::inf_is_inbox("oem12.inf"));
        assert!(!InstalledDriver::inf_is_inbox("OEM7.INF"));
        assert!(InstalledDriver::inf_is_inbox("oem"));
    }

    #[test]
    fn status_serializes_tagged() {
        let s = serde_json::to_string(&DeviceStatus::Problem {
            code: ProblemCode(28),
        })
        .unwrap();
        assert_eq!(s, r#"{"kind":"problem","code":28}"#);
        assert_eq!(
            serde_json::to_string(&DeviceStatus::Ok).unwrap(),
            r#"{"kind":"ok"}"#
        );
    }
}
