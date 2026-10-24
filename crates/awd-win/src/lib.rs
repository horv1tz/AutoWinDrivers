//! Windows back end for AutoWinDrivers.
//!
//! Implements the `awd-core` traits on top of the Windows Configuration
//! Manager (CfgMgr32). Every `unsafe` FFI call in the project lives in this
//! crate behind safe wrappers. On other platforms the crate compiles to an
//! empty shell so that front ends can depend on it unconditionally.

#[cfg(windows)]
mod cfgmgr;
pub mod props;

#[cfg(windows)]
pub use cfgmgr::CfgMgrDeviceSource;

use awd_core::DeviceSource;

/// The default device source for this platform, or `None` where unsupported.
pub fn default_device_source() -> Option<Box<dyn DeviceSource>> {
    #[cfg(windows)]
    {
        Some(Box::new(CfgMgrDeviceSource))
    }
    #[cfg(not(windows))]
    {
        None
    }
}
