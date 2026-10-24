//! Platform-independent core of AutoWinDrivers.
//!
//! This crate knows nothing about Windows APIs. It defines the device model,
//! hardware-ID parsing, the Plug and Play problem-code knowledge base, driver
//! version comparison and the scan report that every front end (CLI, desktop
//! UI, tray agent) consumes. Platform back ends implement [`DeviceSource`].

pub mod device;
pub mod hwid;
pub mod problem;
pub mod report;
pub mod source;
pub mod version;

pub use device::{Device, DeviceStatus, InstalledDriver};
pub use hwid::HardwareId;
pub use problem::{Lang, ProblemCode, Severity, SuggestedAction};
pub use report::{HostInfo, ScanReport, ScanSummary};
pub use source::{DeviceSource, ScanError, ScanOptions};
pub use version::{DriverVersion, ParseVersionError};
