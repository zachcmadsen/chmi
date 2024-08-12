use std::{
    ffi::{CStr, CString},
    mem, ptr, slice,
};

use anyhow::{anyhow, Context};
use windows::Win32::{
    Devices::Display::{
        GetNumberOfPhysicalMonitorsFromHMONITOR,
        GetPhysicalMonitorsFromHMONITOR, PHYSICAL_MONITOR,
    },
    Foundation::{BOOL, HANDLE, LPARAM, RECT, TRUE},
    Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoA, HDC, HMONITOR, MONITORINFOEXA,
    },
};

fn get_physical_monitor(monitor: HMONITOR) -> anyhow::Result<HANDLE> {
    unsafe {
        let mut num_physical_monitors: u32 = 0;
        GetNumberOfPhysicalMonitorsFromHMONITOR(
            monitor,
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
            monitor,
            slice::from_raw_parts_mut(ptr::addr_of_mut!(physical_monitor), 1),
        )
        .context("failed to get physical monitors")?;

        Ok(physical_monitor.hPhysicalMonitor)
    }
}

fn get_device_name(monitor: HMONITOR) -> anyhow::Result<CString> {
    unsafe {
        let mut monitor_info = MONITORINFOEXA::default();
        monitor_info.monitorInfo.cbSize =
            mem::size_of_val(&monitor_info) as u32;
        GetMonitorInfoA(monitor, ptr::addr_of_mut!(monitor_info) as _)
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

fn get_monitor(monitor: HMONITOR) -> anyhow::Result<Monitor> {
    let physical_monitor = get_physical_monitor(monitor)?;
    let device_name = get_device_name(monitor)?;

    // TODO:
    // 1. Get the device ID using EnumDisplayDevices and the device name
    // 2. Get the monitor's friendly name with QueryDisplayConfig and the device ID
    // 3. Get the monitor's capability string and parse it
    // 4. Return the monitor? I think the struct only needs the physical monitor handle and the capabilities
    Ok(todo!())
}

unsafe extern "system" fn enum_display_monitors_callback(
    monitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let _monitor = match get_monitor(monitor) {
        Ok(_monitor) => todo!("push the monitor to a vec or something"),
        Err(err) => {
            eprintln!("failed to get monitor: {}", err);
            return TRUE;
        }
    };

    // Return TRUE to continue the enumeration.
    TRUE
}

pub struct Monitor {}

pub fn get_monitors() -> anyhow::Result<Vec<Monitor>> {
    unsafe {
        // Pass None, i.e., NULL, for the first two parameters to enumerate
        // all display monitors.
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_display_monitors_callback),
            None,
        )
        .ok()?;
    }

    Ok(Vec::new())
}
