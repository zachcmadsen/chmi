use std::{
    collections::HashMap,
    ffi::{CStr, OsString},
    mem,
    os::windows::ffi::OsStringExt,
    ptr, slice,
};

use anyhow::{anyhow, bail, Context};
use tracing::error;
use windows::{
    core::PCSTR,
    Win32::{
        Devices::Display::{
            CapabilitiesRequestAndCapabilitiesReply, DestroyPhysicalMonitor,
            DisplayConfigGetDeviceInfo, GetCapabilitiesStringLength,
            GetDisplayConfigBufferSizes,
            GetNumberOfPhysicalMonitorsFromHMONITOR,
            GetPhysicalMonitorsFromHMONITOR, GetVCPFeatureAndVCPFeatureReply,
            QueryDisplayConfig, SetVCPFeature,
            DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            DISPLAYCONFIG_TARGET_DEVICE_NAME, PHYSICAL_MONITOR,
        },
        Foundation::{BOOL, FALSE, HANDLE, LPARAM, RECT, TRUE},
        Graphics::Gdi::{
            EnumDisplayDevicesA, EnumDisplayMonitors, GetMonitorInfoA,
            DISPLAY_DEVICEA, HDC, HMONITOR, MONITORINFOEXA,
        },
    },
};

use crate::{
    cache::CapabilitiesCache,
    cap::{Capabilities, Input, INPUT_SELECT_CODE},
    parse,
};

fn string_from_wide(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(0);
    OsString::from_wide(&wide[..len])
        .to_str()
        .expect("WCHAR strings from the OS should be valid Unicode")
        .to_string()
}

/// Returns a map of device IDs to friendly names for all display devices.
fn get_friendly_name_map() -> anyhow::Result<HashMap<String, String>> {
    unsafe {
        let mut num_paths = 0;
        let mut num_modes = 0;
        GetDisplayConfigBufferSizes(
            windows::Win32::Devices::Display::QDC_ONLY_ACTIVE_PATHS,
            ptr::addr_of_mut!(num_paths),
            ptr::addr_of_mut!(num_modes),
        )
        .ok()
        .context(
            "failed to get buffer sizes for display device configurations",
        )?;

        let mut paths = Vec::with_capacity(num_paths as usize);
        let mut modes = Vec::with_capacity(num_modes as usize);

        QueryDisplayConfig(
            windows::Win32::Devices::Display::QDC_ONLY_ACTIVE_PATHS,
            ptr::addr_of_mut!(num_paths),
            paths.as_mut_ptr(),
            ptr::addr_of_mut!(num_modes),
            modes.as_mut_ptr(),
            None,
        )
        .ok()
        .context("failed to get display device configurations")?;

        paths.set_len(num_paths as usize);
        modes.set_len(num_modes as usize);

        let mut map = HashMap::new();

        for path in paths {
            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target.header.adapterId = path.targetInfo.adapterId;
            target.header.id = path.targetInfo.id;
            target.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            target.header.size = mem::size_of_val(&target) as u32;

            // TODO: Use the actual success error code instead of hardcoding
            // its value.
            if DisplayConfigGetDeviceInfo(ptr::addr_of_mut!(target.header))
                != 0
            {
                bail!(
                    "failed to get display device configuration information"
                );
            }

            let device_id = string_from_wide(&target.monitorDevicePath);
            let friendly_name =
                string_from_wide(&target.monitorFriendlyDeviceName);

            map.insert(device_id, friendly_name);
        }

        Ok(map)
    }
}

/// Returns the device ID of the display monitor associated with an HMONITOR
/// handle.
fn get_device_id(hmonitor: HMONITOR) -> anyhow::Result<String> {
    unsafe {
        let mut monitor_info = MONITORINFOEXA::default();
        monitor_info.monitorInfo.cbSize =
            mem::size_of_val(&monitor_info) as u32;
        GetMonitorInfoA(hmonitor, ptr::addr_of_mut!(monitor_info) as _)
            .ok()
            .context("failed to get the device name for a display monitor")?;

        let device_name_bytes = slice::from_raw_parts(
            monitor_info.szDevice.as_ptr() as _,
            monitor_info.szDevice.len(),
        );
        // The documentation for MONITORINFOEXA doesn't say that the string in
        // szDevice is null-terminated. Because the MONITORINFOEXA struct is
        // zeroed, it's effectively null-terminated when the name is less than
        // 32 characters (the size of szDevice). Hopefully device names are
        // always less than 32 characters...
        let device_name = CStr::from_bytes_until_nul(device_name_bytes)
            .expect("display monitor device names should be null-terminated");

        let mut display_device = DISPLAY_DEVICEA::default();
        display_device.cb = mem::size_of::<DISPLAY_DEVICEA>() as u32;

        EnumDisplayDevicesA(
            PCSTR::from_raw(device_name.as_ptr() as *const u8),
            0,
            ptr::addr_of_mut!(display_device),
            1,
        )
        .ok()
        .context("failed to get the device ID for a display monitor")?;

        let device_id_bytes = slice::from_raw_parts(
            display_device.DeviceID.as_ptr() as _,
            display_device.DeviceID.len(),
        );
        // See the comment above about null-terminated strings for szDevice.
        let device_id = CStr::from_bytes_until_nul(device_id_bytes)
            .expect("display device IDs should be null-terminated");

        Ok(device_id
            .to_str()
            .expect("display device IDs should be valid UTF-8")
            .to_owned())
    }
}

/// Returns the physical monitor associated with an HMONITOR handle.
///
/// # Errors
/// Returns `Err` if there are zero or multiple physical monitors associated
/// with a handle.
fn get_physical_monitor(hmonitor: HMONITOR) -> anyhow::Result<HANDLE> {
    unsafe {
        let mut num_physical_monitors: u32 = 0;
        GetNumberOfPhysicalMonitorsFromHMONITOR(
            hmonitor,
            ptr::addr_of_mut!(num_physical_monitors),
        )
        .context("failed to get the number of physical monitors for a display monitor")?;

        if num_physical_monitors == 0 {
            bail!("display monitor has no associated physical monitor");
        } else if num_physical_monitors > 1 {
            // I don't know what it means for a display to have multiple
            // physical monitors. For example, which one would set I VCP codes
            // on? This is probably a valid scenario, but it's easier to leave
            // it unhandled for now.
            bail!(
                "display monitor has more than one associated physical monitor"
            );
        }

        let mut physical_monitor = PHYSICAL_MONITOR::default();
        GetPhysicalMonitorsFromHMONITOR(
            hmonitor,
            slice::from_raw_parts_mut(ptr::addr_of_mut!(physical_monitor), 1),
        )
        .context("failed to get the physical monitor for a display monitor")?;

        Ok(physical_monitor.hPhysicalMonitor)
    }
}

fn get_capabilities_string(
    handle: &HANDLE,
    device_id: &str,
) -> anyhow::Result<String> {
    unsafe {
        let cache = CapabilitiesCache::new();
        if let Ok(ref cache) = cache {
            if let Ok(Some(capabilities_string)) = cache.get(device_id) {
                return Ok(capabilities_string);
            }
        }

        let mut capabilities_string_len: u32 = 0;
        if GetCapabilitiesStringLength(
            *handle,
            ptr::addr_of_mut!(capabilities_string_len),
        ) == FALSE.0
        {
            bail!("failed to get capabilities string length");
        }

        // TODO: Add retries for capabilities functions failures. I've seen
        // transient failures on my machine.
        if capabilities_string_len == 0 {
            bail!("received an empty capabilities string");
        }

        let mut capabilities_string_bytes =
            Vec::with_capacity(capabilities_string_len as usize);
        if CapabilitiesRequestAndCapabilitiesReply(
            *handle,
            slice::from_raw_parts_mut(
                capabilities_string_bytes.as_mut_ptr(),
                capabilities_string_len as usize,
            ),
        ) == FALSE.0
        {
            bail!("failed to get capabilities string");
        }

        capabilities_string_bytes.set_len(capabilities_string_len as usize);

        let capabilities_string =
            CStr::from_bytes_until_nul(&capabilities_string_bytes)
                .expect("capabilities strings should be null-terminated")
                .to_str()
                .context("capabilities string contains invalid UTF-8")?
                .to_owned();

        if let Ok(ref cache) = cache {
            let _ = cache.set(&device_id, &capabilities_string);
        }

        Ok(capabilities_string)
    }
}

pub struct Monitor {
    handle: HANDLE,
    name: String,
    capabilities: Capabilities,
}

impl Monitor {
    fn new(
        hmonitor: HMONITOR,
        friendly_name_map: &HashMap<String, String>,
    ) -> anyhow::Result<Monitor> {
        let physical_monitor = get_physical_monitor(hmonitor)?;

        let device_id = get_device_id(hmonitor)?;

        let friendly_name = friendly_name_map.get(&device_id).unwrap();

        let capabilities_string =
            get_capabilities_string(&physical_monitor, &device_id)?;

        let capabilities =
            parse::parse_capabilities_string(&capabilities_string)?;

        Ok(Monitor {
            handle: physical_monitor,
            name: friendly_name.clone(),
            capabilities,
        })
    }
}

impl crate::monitor::Monitor for Monitor {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn input(&self) -> anyhow::Result<Input> {
        let mut value = 0;
        unsafe {
            if GetVCPFeatureAndVCPFeatureReply(
                self.handle,
                INPUT_SELECT_CODE,
                None,
                ptr::addr_of_mut!(value),
                None,
            ) == FALSE.0
            {
                bail!(
                    "failed to retrieve the value of VCP code {} for monitor '{}'",
                    INPUT_SELECT_CODE, self.name
                );
            }
        }

        Ok((value as u8)
            .try_into()
            .expect("the value of a VCP code should be valid"))
    }

    fn set_input(&mut self, input: Input) -> anyhow::Result<()> {
        let value: u8 = input.into();
        unsafe {
            // TODO: Use GetLastError to get more error information. Same
            // thing for GetVCPFeatureAndVCPFeatureReply. See BOOL::ok for
            // a possible implementation.
            if SetVCPFeature(self.handle, INPUT_SELECT_CODE, value as u32)
                == FALSE.0
            {
                bail!(
                    "failed to set VCP code {} to {} for monitor '{}'",
                    INPUT_SELECT_CODE,
                    value,
                    self.name
                );
            }
        }

        Ok(())
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyPhysicalMonitor(self.handle);
        }
    }
}

unsafe extern "system" fn enum_display_monitors_callback(
    hmonitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let context = &mut *(data.0 as *mut EnumDisplayMonitorContext);

    match Monitor::new(hmonitor, &context.friendly_name_map) {
        Ok(monitor) => {
            // let monitors = &mut *(data.0 as *mut Vec<Monitor>);
            context.monitors.push(monitor);
        }
        Err(err) => {
            error!("an error occurred while getting display monitor information: {}", err);
            error!("{}", err.root_cause());
        }
    };

    // Return TRUE to continue the enumeration.
    TRUE
}

struct EnumDisplayMonitorContext {
    monitors: Vec<Monitor>,
    friendly_name_map: HashMap<String, String>,
}

pub fn get_monitors() -> anyhow::Result<Vec<Monitor>> {
    let map = get_friendly_name_map().unwrap();

    let mut context = EnumDisplayMonitorContext {
        monitors: Vec::new(),
        friendly_name_map: map,
    };

    unsafe {
        // Pass None, i.e., NULL, for the first two parameters to enumerate
        // all display monitors.
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_display_monitors_callback),
            LPARAM(ptr::addr_of_mut!(context) as _),
        )
        .ok()
        .context("failed to enumerate display monitors")?;
    }

    Ok(context.monitors)
}
