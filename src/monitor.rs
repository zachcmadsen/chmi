use std::{
    ffi::{CStr, CString, OsString},
    mem,
    os::windows::ffi::OsStringExt,
    ptr, slice,
};

use anyhow::{anyhow, Context};
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
    cache,
    cap::{Capabilities, Input},
    parse,
};

fn get_physical_monitor(hmonitor: HMONITOR) -> anyhow::Result<HANDLE> {
    unsafe {
        let mut num_physical_monitors: u32 = 0;
        GetNumberOfPhysicalMonitorsFromHMONITOR(
            hmonitor,
            ptr::addr_of_mut!(num_physical_monitors),
        )
        .context("failed to get number of physical monitors")?;

        if num_physical_monitors != 1 {
            return Err(anyhow!(
                "monitor didn't have exactly one associated physical monitor"
            ));
        }

        let mut physical_monitor = PHYSICAL_MONITOR::default();
        GetPhysicalMonitorsFromHMONITOR(
            hmonitor,
            slice::from_raw_parts_mut(ptr::addr_of_mut!(physical_monitor), 1),
        )
        .context("failed to get physical monitors")?;

        Ok(physical_monitor.hPhysicalMonitor)
    }
}

fn get_device_name(hmonitor: HMONITOR) -> anyhow::Result<CString> {
    unsafe {
        let mut monitor_info = MONITORINFOEXA::default();
        monitor_info.monitorInfo.cbSize =
            mem::size_of_val(&monitor_info) as u32;
        GetMonitorInfoA(hmonitor, ptr::addr_of_mut!(monitor_info) as _)
            .ok()
            .context("failed to get monitor information")?;

        let device_name_bytes = slice::from_raw_parts(
            monitor_info.szDevice.as_ptr() as _,
            monitor_info.szDevice.len(),
        );
        // The documentation doesn't say whether the string in szDevice is
        // null-terminated. Because the MONITORINFOEXA struct is zeroed, it
        // ends up null-terminated when the device name is less than 32
        // characters (the size of szDevice).
        //
        // https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-monitorinfoexa
        let device_name = CStr::from_bytes_until_nul(device_name_bytes)
            .expect("monitor device names should be null-terminated");

        Ok(device_name.to_owned())
    }
}

fn get_device_id(device_name: &CStr) -> anyhow::Result<CString> {
    unsafe {
        let mut display_device = DISPLAY_DEVICEA::default();
        display_device.cb = mem::size_of::<DISPLAY_DEVICEA>() as u32;

        EnumDisplayDevicesA(
            PCSTR::from_raw(device_name.as_ptr() as *const u8),
            0,
            ptr::addr_of_mut!(display_device),
            1,
        )
        .ok()
        .context("failed to get display devices")?;

        let device_id_bytes = slice::from_raw_parts(
            display_device.DeviceID.as_ptr() as _,
            display_device.DeviceID.len(),
        );

        let device_id = CStr::from_bytes_until_nul(device_id_bytes)
            .expect("display device IDs should be null-terminated");

        Ok(device_id.to_owned())
    }
}

fn os_string_from_wchar_str(wchar_str: &[u16]) -> OsString {
    let len = wchar_str.iter().position(|&c| c == 0).unwrap_or(0);
    OsString::from_wide(&wchar_str[..len])
}

fn get_friendly_device_name(device_id: &CStr) -> anyhow::Result<String> {
    unsafe {
        let mut num_paths = 0;
        let mut num_modes = 0;
        GetDisplayConfigBufferSizes(
            windows::Win32::Devices::Display::QDC_ONLY_ACTIVE_PATHS,
            ptr::addr_of_mut!(num_paths),
            ptr::addr_of_mut!(num_modes),
        )
        .ok()
        .context("failed to get buffer sizes for QueryDisplayConfig")?;

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
        .context("failed to get display path information")?;

        paths.set_len(num_paths as usize);
        modes.set_len(num_modes as usize);

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
                return Err(anyhow!(
                    "failed to get display configuration info"
                ));
            }

            let device_path =
                os_string_from_wchar_str(&target.monitorDevicePath);

            // The device path of the associated target device is the same as
            // the device ID from DISPLAY_DEVICEA.
            if device_path.as_encoded_bytes() != device_id.to_bytes() {
                continue;
            }

            let friendly_device_name =
                os_string_from_wchar_str(&target.monitorFriendlyDeviceName)
                    .to_string_lossy()
                    .to_string();

            return Ok(friendly_device_name);
        }

        Err(anyhow!(
            "no display path with ID '{}'",
            device_id.to_string_lossy()
        ))
    }
}

fn get_capabilities_string(handle: &HANDLE) -> anyhow::Result<String> {
    unsafe {
        let mut capabilities_string_len: u32 = 0;
        if GetCapabilitiesStringLength(
            *handle,
            ptr::addr_of_mut!(capabilities_string_len),
        ) == FALSE.0
        {
            return Err(anyhow!("failed to get a capabilities string length"));
        }

        // TODO: Add retries for capabilities functions failures. I've seen
        // transient failures on my machine.
        if capabilities_string_len == 0 {
            return Err(anyhow!("received an empty capabilities string"));
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
            return Err(anyhow!("failed to get a capabilities string"));
        }
        capabilities_string_bytes.set_len(capabilities_string_len as usize);

        let capabilities_string =
            CStr::from_bytes_until_nul(&capabilities_string_bytes)
                .expect("capabilities strings should be null-terminated");

        Ok(capabilities_string
            .to_str()
            .context("capabilities string contains invalid UTF-8")?
            .to_owned())
    }
}

unsafe extern "system" fn enum_display_monitors_callback(
    hmonitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    data: LPARAM,
) -> BOOL {
    match Monitor::from_hmonitor(hmonitor) {
        Ok(monitor) => {
            let monitors = &mut *(data.0 as *mut Vec<Monitor>);
            monitors.push(monitor);
        }
        Err(err) => {
            eprintln!("failed to get monitor: {}", err);
            return TRUE;
        }
    };

    // Return TRUE to continue the enumeration.
    TRUE
}

pub struct Monitor {
    handle: HANDLE,
    name: String,
    capabilities: Capabilities,
}

impl Drop for Monitor {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyPhysicalMonitor(self.handle);
        }
    }
}

impl Monitor {
    fn from_hmonitor(hmonitor: HMONITOR) -> anyhow::Result<Monitor> {
        let physical_monitor = get_physical_monitor(hmonitor)?;
        let device_name = get_device_name(hmonitor)?;
        let device_id = get_device_id(&device_name)?;
        let friendly_device_name = get_friendly_device_name(&device_id)?;

        // eprintln!("checking capabilities cache...");
        let capabilities_string = match cache::get(device_id.to_str()?)? {
            Some(capabilities_string) => capabilities_string,
            None => {
                // eprintln!(
                //     "capabilities cache miss, fetching capabilities strings"
                // );
                let capabilities_string =
                    get_capabilities_string(&physical_monitor)?;
                cache::set(device_id.to_str()?, &capabilities_string)?;
                capabilities_string
            }
        };

        let capabilities =
            parse::parse_capabilities_string(&capabilities_string)?;

        Ok(Monitor {
            handle: physical_monitor,
            name: friendly_device_name,
            capabilities,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    pub fn input(&self) -> anyhow::Result<Input> {
        let mut current = 0;
        unsafe {
            if GetVCPFeatureAndVCPFeatureReply(
                self.handle,
                0x60,
                None,
                ptr::addr_of_mut!(current),
                None,
            ) == FALSE.0
            {
                return Err(anyhow!("failed to get blah"));
            }
        }

        match current {
            0x0F => Ok(Input::DisplayPort1),
            0x10 => Ok(Input::DisplayPort2),
            0x11 => Ok(Input::Hdmi1),
            0x12 => Ok(Input::Hdmi2),
            _ => Err(anyhow!("invalid input vcp value")),
        }
    }

    pub fn set_input(&self, input: &Input) -> anyhow::Result<()> {
        let value = match input {
            Input::DisplayPort1 => 0x0F,
            Input::DisplayPort2 => 0x10,
            Input::Hdmi1 => 0x11,
            Input::Hdmi2 => 0x12,
        };

        unsafe {
            if SetVCPFeature(self.handle, 0x60, value) == FALSE.0 {
                return Err(anyhow!("failed to get blah"));
            }
        }

        Ok(())
    }
}

pub fn get_monitors() -> anyhow::Result<Vec<Monitor>> {
    let mut monitors: Vec<Monitor> = Vec::new();

    unsafe {
        // Pass None, i.e., NULL, for the first two parameters to enumerate
        // all display monitors.
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_display_monitors_callback),
            LPARAM(ptr::addr_of_mut!(monitors) as _),
        )
        .ok()?;
    }

    Ok(monitors)
}
