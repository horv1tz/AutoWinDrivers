//! Runs against the real machine; only meaningful on Windows (CI uses `windows-latest`).
#![cfg(windows)]

use awd_core::{DeviceSource, ScanOptions};
use awd_win::CfgMgrDeviceSource;

#[test]
fn live_scan_returns_devices_with_ids() {
    let devices = CfgMgrDeviceSource
        .scan(&ScanOptions::default())
        .expect("scan succeeds");
    assert!(!devices.is_empty(), "a Windows machine always has devices");
    for d in &devices {
        assert!(!d.instance_id.is_empty());
        assert!(!d.name.is_empty());
        assert!(
            d.present,
            "present-only scan returned a phantom: {}",
            d.instance_id
        );
    }
    assert!(
        devices.iter().any(|d| d.driver.is_some()),
        "at least one device has a driver bound"
    );
}

#[test]
fn scan_with_phantoms_is_a_superset() {
    let present = CfgMgrDeviceSource.scan(&ScanOptions::default()).unwrap();
    let all = CfgMgrDeviceSource
        .scan(&ScanOptions {
            include_not_present: true,
        })
        .unwrap();
    assert!(all.len() >= present.len());
}
