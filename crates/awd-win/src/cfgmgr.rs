//! Device enumeration through the Configuration Manager API (`cfgmgr32.dll`).
//!
//! CfgMgr32 is preferred over SetupAPI here: it needs no device-info-set
//! handle, reads the same unified device properties (`DEVPKEY_*`) and can
//! address phantom (not currently present) devices.

use awd_core::{
    Device, DeviceSource, DeviceStatus, HardwareId, InstalledDriver, ScanError, ScanOptions,
};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_DEVNODE_STATUS_FLAGS, CM_GETIDLIST_FILTER_NONE, CM_GETIDLIST_FILTER_PRESENT,
    CM_Get_DevNode_PropertyW, CM_Get_DevNode_Status, CM_Get_Device_ID_List_SizeW,
    CM_Get_Device_ID_ListW, CM_LOCATE_DEVNODE_PHANTOM, CM_Locate_DevNodeW, CM_PROB, CONFIGRET,
    CR_BUFFER_SMALL, CR_NO_SUCH_DEVNODE, CR_NO_SUCH_VALUE, CR_SUCCESS,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_BusReportedDeviceDesc, DEVPKEY_Device_Class, DEVPKEY_Device_ClassGuid,
    DEVPKEY_Device_CompatibleIds, DEVPKEY_Device_DeviceDesc, DEVPKEY_Device_DriverDate,
    DEVPKEY_Device_DriverDesc, DEVPKEY_Device_DriverInfPath, DEVPKEY_Device_DriverInfSection,
    DEVPKEY_Device_DriverProvider, DEVPKEY_Device_DriverRank, DEVPKEY_Device_DriverVersion,
    DEVPKEY_Device_EnumeratorName, DEVPKEY_Device_FriendlyName, DEVPKEY_Device_HardwareIds,
    DEVPKEY_Device_IsPresent, DEVPKEY_Device_LocationInfo, DEVPKEY_Device_Manufacturer,
    DEVPKEY_Device_MatchingDeviceId, DEVPKEY_Device_Parent, DEVPROP_TYPE_BOOLEAN,
    DEVPROP_TYPE_FILETIME, DEVPROP_TYPE_GUID, DEVPROP_TYPE_STRING, DEVPROP_TYPE_STRING_LIST,
    DEVPROP_TYPE_UINT32, DEVPROPTYPE,
};
use windows::Win32::Foundation::DEVPROPKEY;
use windows::core::PCWSTR;

use crate::props;

/// Enumerates devices with CfgMgr32. Needs no special privileges.
#[derive(Debug, Default, Clone, Copy)]
pub struct CfgMgrDeviceSource;

impl DeviceSource for CfgMgrDeviceSource {
    fn scan(&self, options: &ScanOptions) -> Result<Vec<Device>, ScanError> {
        let ids = device_instance_ids(!options.include_not_present)?;
        let mut devices = Vec::with_capacity(ids.len());
        for id in ids {
            // The root of the device tree is not a device; Device Manager hides it too.
            if id.len() >= 6 && id[..6].eq_ignore_ascii_case("HTREE\\") {
                continue;
            }
            if let Some(node) = DevNode::locate(&id)? {
                devices.push(node.read_device(id)?);
            }
        }
        Ok(devices)
    }
}

fn check(api: &'static str, ret: CONFIGRET) -> Result<(), ScanError> {
    if ret == CR_SUCCESS {
        Ok(())
    } else {
        Err(ScanError::CfgMgr { api, code: ret.0 })
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// All device instance ids, present-only or including phantoms.
fn device_instance_ids(present_only: bool) -> Result<Vec<String>, ScanError> {
    let flags = if present_only {
        CM_GETIDLIST_FILTER_PRESENT
    } else {
        CM_GETIDLIST_FILTER_NONE
    };
    // The list can grow between the size query and the fetch (hot-plug), in
    // which case the fetch reports CR_BUFFER_SMALL and we simply retry.
    for _ in 0..8 {
        let mut len = 0u32;
        // SAFETY: `len` is a valid out-pointer; a null filter with these flags is documented.
        let ret = unsafe { CM_Get_Device_ID_List_SizeW(&mut len, PCWSTR::null(), flags) };
        check("CM_Get_Device_ID_List_SizeW", ret)?;

        let mut buf = vec![0u16; len as usize];
        // SAFETY: `buf` is a writable buffer whose length is passed to the API.
        let ret = unsafe { CM_Get_Device_ID_ListW(PCWSTR::null(), &mut buf, flags) };
        if ret == CR_BUFFER_SMALL {
            continue;
        }
        check("CM_Get_Device_ID_ListW", ret)?;

        let bytes: Vec<u8> = buf.iter().flat_map(|u| u.to_le_bytes()).collect();
        return Ok(props::utf16_multi_sz(&bytes));
    }
    Err(ScanError::Other(
        "device list kept changing while it was being read".to_owned(),
    ))
}

/// A located device node (`DEVINST`).
struct DevNode(u32);

impl DevNode {
    /// Locates a device by instance id, including phantoms. `Ok(None)` when it
    /// vanished between enumeration and lookup.
    fn locate(instance_id: &str) -> Result<Option<Self>, ScanError> {
        let wide = to_wide(instance_id);
        let mut inst = 0u32;
        // SAFETY: `inst` is a valid out-pointer; `wide` is NUL-terminated and outlives the call.
        let ret = unsafe {
            CM_Locate_DevNodeW(&mut inst, PCWSTR(wide.as_ptr()), CM_LOCATE_DEVNODE_PHANTOM)
        };
        if ret == CR_NO_SUCH_DEVNODE {
            return Ok(None);
        }
        check("CM_Locate_DevNodeW", ret)?;
        Ok(Some(Self(inst)))
    }

    /// Reads a raw property. `Ok(None)` when the property is not set.
    fn raw(&self, key: &DEVPROPKEY) -> Result<Option<(DEVPROPTYPE, Vec<u8>)>, ScanError> {
        let mut ty = DEVPROPTYPE(0);
        let mut size = 0u32;
        // SAFETY: all pointers are valid for the duration of the call; a null
        // buffer with size 0 is the documented way to query the required size.
        let ret = unsafe { CM_Get_DevNode_PropertyW(self.0, key, &mut ty, None, &mut size, 0) };
        match ret {
            CR_NO_SUCH_VALUE => return Ok(None),
            CR_BUFFER_SMALL | CR_SUCCESS if size > 0 => {}
            CR_SUCCESS => return Ok(None),
            other => check("CM_Get_DevNode_PropertyW", other)?,
        }
        let mut buf = vec![0u8; size as usize];
        // SAFETY: `buf` has exactly `size` writable bytes, as passed in `size`.
        let ret = unsafe {
            CM_Get_DevNode_PropertyW(self.0, key, &mut ty, Some(buf.as_mut_ptr()), &mut size, 0)
        };
        if ret == CR_NO_SUCH_VALUE {
            return Ok(None);
        }
        check("CM_Get_DevNode_PropertyW", ret)?;
        buf.truncate(size as usize);
        Ok(Some((ty, buf)))
    }

    fn typed(&self, key: &DEVPROPKEY, want: DEVPROPTYPE) -> Result<Option<Vec<u8>>, ScanError> {
        Ok(self
            .raw(key)?
            .filter(|(ty, _)| *ty == want)
            .map(|(_, bytes)| bytes))
    }

    fn string(&self, key: &DEVPROPKEY) -> Result<Option<String>, ScanError> {
        Ok(self
            .typed(key, DEVPROP_TYPE_STRING)?
            .map(|b| props::utf16_string(&b))
            .filter(|s| !s.is_empty()))
    }

    fn string_list(&self, key: &DEVPROPKEY) -> Result<Vec<String>, ScanError> {
        Ok(self
            .typed(key, DEVPROP_TYPE_STRING_LIST)?
            .map(|b| props::utf16_multi_sz(&b))
            .unwrap_or_default())
    }

    fn u32(&self, key: &DEVPROPKEY) -> Result<Option<u32>, ScanError> {
        Ok(self
            .typed(key, DEVPROP_TYPE_UINT32)?
            .and_then(|b| props::u32_le(&b)))
    }

    fn boolean(&self, key: &DEVPROPKEY) -> Result<Option<bool>, ScanError> {
        Ok(self
            .typed(key, DEVPROP_TYPE_BOOLEAN)?
            .and_then(|b| props::boolean(&b)))
    }

    fn guid(&self, key: &DEVPROPKEY) -> Result<Option<uuid::Uuid>, ScanError> {
        Ok(self
            .typed(key, DEVPROP_TYPE_GUID)?
            .and_then(|b| props::guid(&b)))
    }

    fn date(&self, key: &DEVPROPKEY) -> Result<Option<chrono::NaiveDate>, ScanError> {
        Ok(self
            .typed(key, DEVPROP_TYPE_FILETIME)?
            .and_then(|b| props::filetime_date(&b)))
    }

    /// `(DN_* flags, problem number)`, or `None` for a phantom device.
    fn status(&self) -> Option<(u32, u32)> {
        let mut flags = CM_DEVNODE_STATUS_FLAGS(0);
        let mut problem = CM_PROB(0);
        // SAFETY: both out-pointers are valid for the call.
        let ret = unsafe { CM_Get_DevNode_Status(&mut flags, &mut problem, self.0, 0) };
        (ret == CR_SUCCESS).then_some((flags.0, problem.0))
    }

    fn read_device(&self, instance_id: String) -> Result<Device, ScanError> {
        let status = self.status();
        let present = self
            .boolean(&DEVPKEY_Device_IsPresent)?
            .unwrap_or(status.is_some());

        let description = self
            .string(&DEVPKEY_Device_DeviceDesc)?
            .or(self.string(&DEVPKEY_Device_BusReportedDeviceDesc)?);
        let name = self
            .string(&DEVPKEY_Device_FriendlyName)?
            .or_else(|| description.clone())
            .unwrap_or_else(|| instance_id.clone());

        let hardware_ids = self
            .string_list(&DEVPKEY_Device_HardwareIds)?
            .into_iter()
            .map(HardwareId::new)
            .collect();
        let compatible_ids = self
            .string_list(&DEVPKEY_Device_CompatibleIds)?
            .into_iter()
            .map(HardwareId::new)
            .collect();

        let driver = self.read_driver()?;

        Ok(Device {
            instance_id,
            name,
            description,
            class: self.string(&DEVPKEY_Device_Class)?,
            class_guid: self.guid(&DEVPKEY_Device_ClassGuid)?,
            manufacturer: self.string(&DEVPKEY_Device_Manufacturer)?,
            enumerator: self.string(&DEVPKEY_Device_EnumeratorName)?,
            hardware_ids,
            compatible_ids,
            location: self.string(&DEVPKEY_Device_LocationInfo)?,
            parent: self.string(&DEVPKEY_Device_Parent)?,
            present,
            devnode_status: status.map(|(flags, _)| flags),
            status: DeviceStatus::from_cm(present, status.map(|(_, problem)| problem)),
            driver,
        })
    }

    fn read_driver(&self) -> Result<Option<InstalledDriver>, ScanError> {
        let inf_path = self.string(&DEVPKEY_Device_DriverInfPath)?;
        let version_raw = self.string(&DEVPKEY_Device_DriverVersion)?;
        if inf_path.is_none() && version_raw.is_none() {
            return Ok(None);
        }
        let version = version_raw.as_deref().and_then(|v| v.parse().ok());
        Ok(Some(InstalledDriver {
            provider: self.string(&DEVPKEY_Device_DriverProvider)?,
            description: self.string(&DEVPKEY_Device_DriverDesc)?,
            version,
            version_raw: if version.is_some() { None } else { version_raw },
            date: self.date(&DEVPKEY_Device_DriverDate)?,
            is_inbox: inf_path
                .as_deref()
                .is_some_and(InstalledDriver::inf_is_inbox),
            inf_path,
            inf_section: self.string(&DEVPKEY_Device_DriverInfSection)?,
            matching_device_id: self
                .string(&DEVPKEY_Device_MatchingDeviceId)?
                .map(HardwareId::new),
            rank: self.u32(&DEVPKEY_Device_DriverRank)?,
        }))
    }
}
