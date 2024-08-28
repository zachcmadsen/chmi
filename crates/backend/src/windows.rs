use std::{
    ffi::{CStr, OsString},
    mem,
    os::windows::ffi::OsStringExt,
    ptr, slice,
};

use windows::{
    core::PCSTR,
    Win32::{
        Devices::Display::{
            DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
            GetNumberOfPhysicalMonitorsFromHMONITOR,
            GetPhysicalMonitorsFromHMONITOR, GetVCPFeatureAndVCPFeatureReply,
            QueryDisplayConfig, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_TARGET_DEVICE_NAME,
            PHYSICAL_MONITOR, QDC_ONLY_ACTIVE_PATHS,
        },
        Foundation::{BOOL, ERROR_SUCCESS, FALSE, HANDLE, LPARAM, RECT, TRUE},
        Graphics::Gdi::{
            EnumDisplayDevicesA, EnumDisplayMonitors, GetMonitorInfoA,
            DISPLAY_DEVICEA, HDC, HMONITOR, MONITORINFOEXA,
        },
    },
};

use crate::Error;

impl From<windows::core::Error> for Error {
    fn from(_value: windows::core::Error) -> Self {
        // TODO: Log the error.
        Error::Os
    }
}

const INPUT_SELECT_VCP_CODE: u8 = 0x60;

fn string_from_wide(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(0);
    OsString::from_wide(&wide[..len])
        .to_str()
        .expect("WCHAR strings from the OS should be valid Unicode")
        .to_string()
}

fn get_display_paths() -> Result<Vec<DISPLAYCONFIG_PATH_INFO>, Error> {
    let mut num_paths = 0;
    let mut num_modes = 0;
    // SAFETY:
    // - The flags argument is a valid value.
    // - The pointer arguments aren't null.
    unsafe {
        GetDisplayConfigBufferSizes(
            QDC_ONLY_ACTIVE_PATHS,
            ptr::addr_of_mut!(num_paths),
            ptr::addr_of_mut!(num_modes),
        )
        .ok()
    }?;

    let mut paths = Vec::with_capacity(num_paths as usize);
    let mut modes = Vec::with_capacity(num_modes as usize);

    let prev_num_paths = num_paths;
    let prev_num_modes = num_modes;

    // TODO: The ERROR_INSUFFICIENT_BUFFER return code is recoverable so try
    // to handle it. The "Remarks" section of the QueryDisplayConfig
    // documentation has more information.
    // SAFETY:
    // - The `flags` argument is a valid value.
    // - The pointer arguments aren't null.
    // - The array arguments point to properly sized `Vec`'s.
    unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            ptr::addr_of_mut!(num_paths),
            paths.as_mut_ptr(),
            ptr::addr_of_mut!(num_modes),
            modes.as_mut_ptr(),
            None,
        )
        .ok()
    }?;

    // The updated numbers of elements shouldn't be greater than the initial
    // numbers.
    assert!(num_paths <= prev_num_paths);
    assert!(num_modes <= prev_num_modes);

    // SAFETY:
    // - The new lengths are less than or equal to the initial capacities.
    // - The elements were initialized by `QueryDisplayConfig`.
    unsafe {
        paths.set_len(num_paths as usize);
        modes.set_len(num_modes as usize)
    };

    Ok(paths)
}

fn get_device_id_and_name(path: &DISPLAYCONFIG_PATH_INFO) -> (String, String) {
    let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
    target.header.adapterId = path.targetInfo.adapterId;
    target.header.id = path.targetInfo.id;
    target.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
    target.header.size = mem::size_of_val(&target) as u32;

    let rc = unsafe {
        DisplayConfigGetDeviceInfo(ptr::addr_of_mut!(target.header))
    };
    if rc as u32 != ERROR_SUCCESS.0 {
        todo!()
    }

    let device_id = string_from_wide(&target.monitorDevicePath);
    let device_name = string_from_wide(&target.monitorFriendlyDeviceName);

    (device_id, device_name)
}

fn get_hmonitors() -> Vec<HMONITOR> {
    unsafe extern "system" fn enum_display_monitors_callback(
        hmonitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let hmonitors = &mut *(data.0 as *mut Vec<HMONITOR>);
        hmonitors.push(hmonitor);

        // Return TRUE to continue the enumeration.
        TRUE
    }

    let mut hmonitors = Vec::new();

    unsafe {
        // Pass None, i.e., NULL, for the first two parameters to enumerate
        // all display monitors.
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_display_monitors_callback),
            LPARAM(ptr::addr_of_mut!(hmonitors) as _),
        )
        .ok()
        .unwrap();
    }

    hmonitors
}

fn get_device_id(hmonitor: &HMONITOR) -> String {
    let mut monitor_info = MONITORINFOEXA::default();
    monitor_info.monitorInfo.cbSize = mem::size_of_val(&monitor_info) as u32;
    unsafe {
        GetMonitorInfoA(*hmonitor, ptr::addr_of_mut!(monitor_info) as _)
            .ok()
            .unwrap()
    };

    let device_name_bytes = unsafe {
        slice::from_raw_parts(
            monitor_info.szDevice.as_ptr() as _,
            monitor_info.szDevice.len(),
        )
    };
    // The documentation for MONITORINFOEXA doesn't say that the string in
    // szDevice is null-terminated. Because the MONITORINFOEXA struct is
    // zeroed, it's effectively null-terminated when the name is less than
    // 32 characters (the size of szDevice). Hopefully device names are
    // always less than 32 characters...
    let device_name = CStr::from_bytes_until_nul(device_name_bytes)
        .expect("display monitor device names should be null-terminated");

    let mut display_device = DISPLAY_DEVICEA {
        cb: mem::size_of::<DISPLAY_DEVICEA>() as u32,
        ..DISPLAY_DEVICEA::default()
    };
    unsafe {
        EnumDisplayDevicesA(
            PCSTR::from_raw(device_name.as_ptr() as *const u8),
            0,
            ptr::addr_of_mut!(display_device),
            1,
        )
        .ok()
        .unwrap()
    };

    let device_id_bytes = unsafe {
        slice::from_raw_parts(
            display_device.DeviceID.as_ptr() as _,
            display_device.DeviceID.len(),
        )
    };
    // See the comment above about null-terminated strings for szDevice.
    let device_id = CStr::from_bytes_until_nul(device_id_bytes)
        .expect("display device IDs should be null-terminated");

    device_id
        .to_str()
        .expect("display device IDs should be valid UTF-8")
        .to_owned()
}

fn get_physical_monitor(hmonitor: HMONITOR) -> HANDLE {
    let mut num_physical_monitors: u32 = 0;
    unsafe {
        GetNumberOfPhysicalMonitorsFromHMONITOR(
            hmonitor,
            ptr::addr_of_mut!(num_physical_monitors),
        )
        .unwrap()
    };

    if num_physical_monitors == 0 {
        panic!("display monitor has no associated physical monitor");
    } else if num_physical_monitors > 1 {
        // I don't know what it means for a display to have multiple
        // physical monitors. For example, which one would set I VCP codes
        // on? This is probably a valid scenario, but it's easier to leave
        // it unhandled for now.
        panic!(
            "display monitor has more than one associated physical monitor"
        );
    }

    let mut physical_monitor = PHYSICAL_MONITOR::default();
    unsafe {
        GetPhysicalMonitorsFromHMONITOR(
            hmonitor,
            slice::from_raw_parts_mut(ptr::addr_of_mut!(physical_monitor), 1),
        )
        .unwrap()
    };

    physical_monitor.hPhysicalMonitor
}

pub fn get_display_names() -> Result<Vec<String>, Error> {
    let display_paths = get_display_paths()?;

    let mut names = Vec::new();
    for path in display_paths {
        let (_, name) = get_device_id_and_name(&path);
        names.push(name);
    }

    Ok(names)
}

pub fn get_input(display_name: &str) -> Result<u8, Error> {
    // 1. Validate display_name, i.e., that it shows up in the list from get_display_names.
    // 2. Iterate hmonitors, using their device ID to find the monitor for the given display name.
    // 3. Get the physical montior from the hmonitor
    // 4. (Optional) Get the capabilities string
    // 5. (Optional) Check that it supports the input VCP code
    // 6. Get the value of the VCP code

    // Note, all of the steps except the last one are the same between get and set

    let (id, _name) = get_display_paths()?
        .iter()
        .map(get_device_id_and_name)
        .find(|(_, name)| name == display_name)
        .ok_or(Error::DisplayNotFound(display_name.to_string()))?;

    let hmonitor = get_hmonitors()
        .into_iter()
        .find(|hmonitor| {
            let device_id = get_device_id(hmonitor);
            device_id == id
        })
        .ok_or(Error::DisplayNotFound(display_name.to_string()))?;

    let physical_handle = get_physical_monitor(hmonitor);

    let mut value = 0;
    if unsafe {
        GetVCPFeatureAndVCPFeatureReply(
            physical_handle,
            INPUT_SELECT_VCP_CODE,
            None,
            ptr::addr_of_mut!(value),
            None,
        )
    } == FALSE.0
    {
        panic!(
            "failed to retrieve the value of VCP code {} for monitor '{}'",
            INPUT_SELECT_VCP_CODE, _name
        );
    }

    Ok(value as u8)
}
