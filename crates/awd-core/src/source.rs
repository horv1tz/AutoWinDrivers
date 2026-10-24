//! The platform abstraction for enumerating devices.

use crate::device::Device;

/// Options for a device scan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanOptions {
    /// Also return devices that are known to the system but not connected.
    pub include_not_present: bool,
}

/// Errors from a device scan.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    /// No device source exists for this platform.
    #[error("device scanning is only supported on Windows")]
    Unsupported,
    /// A Configuration Manager (CfgMgr32) call failed.
    #[error("{api} failed with CONFIGRET 0x{code:08X}")]
    CfgMgr {
        /// The API that failed.
        api: &'static str,
        /// The raw `CONFIGRET` value.
        code: u32,
    },
    /// Any other failure.
    #[error("{0}")]
    Other(String),
}

/// Something that can enumerate the devices of a machine.
pub trait DeviceSource {
    /// Enumerates devices according to `options`.
    fn scan(&self, options: &ScanOptions) -> Result<Vec<Device>, ScanError>;
}
