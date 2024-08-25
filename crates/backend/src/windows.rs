use std::{ffi::OsString, mem, os::windows::ffi::OsStringExt, ptr};

use windows::Win32::{
    Devices::Display::{
        DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
        QueryDisplayConfig, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_TARGET_DEVICE_NAME,
        QDC_ONLY_ACTIVE_PATHS,
    },
    Foundation::ERROR_SUCCESS,
};

use crate::Error;

fn string_from_wide(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(0);
    OsString::from_wide(&wide[..len])
        .to_str()
        .expect("WCHAR strings from the OS should be valid Unicode")
        .to_string()
}

fn get_display_paths() -> Vec<DISPLAYCONFIG_PATH_INFO> {
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
        .unwrap()
    };

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
        .unwrap()
    };

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

    paths
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

pub fn get_display_names() -> Vec<String> {
    let display_paths = get_display_paths();

    let mut names = Vec::new();
    for path in display_paths {
        let (_id, name) = get_device_id_and_name(&path);
        names.push(name);
    }

    names
}

pub fn get_input(display_name: &str) -> Result<u8, Error> {
    // 1. Validate display_name, i.e., that it shows up in the list from get_display_names.
    // 2. Iterate hmonitors, using their device ID to find the monitor for the given display name.
    // 3. Get the physical montior from the hmonitor
    // 4. (Optional) Get the capabilities string
    // 5. (Optional) Check that it supports the input VCP code
    // 6. Get the value of the VCP code

    // Note, all of the steps except the last one are the same between get and set

    let display_paths = get_display_paths();

    let mut ids_and_names = Vec::new();
    for path in display_paths {
        let id_and_name = get_device_id_and_name(&path);
        ids_and_names.push(id_and_name);
    }

    if !ids_and_names.iter().any(|(_, name)| name == display_name) {
        return Err(Error::DisplayNotFound(display_name.to_string()));
    }

    Ok(0)
}
