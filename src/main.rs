mod cap;
mod parse;

use std::{ffi::CString, ptr, slice};

use windows::Win32::{
    Devices::Display::{
        CapabilitiesRequestAndCapabilitiesReply, DestroyPhysicalMonitor,
        GetCapabilitiesStringLength, GetNumberOfPhysicalMonitorsFromHMONITOR,
        GetPhysicalMonitorsFromHMONITOR,
    },
    Foundation::{BOOL, HANDLE, LPARAM, RECT, TRUE},
    Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR},
};

unsafe extern "system" fn enum_display_monitors_callback(
    monitor_handle: HMONITOR,
    _: HDC,
    _: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let mut num_physical_monitors: u32 = 0;
    GetNumberOfPhysicalMonitorsFromHMONITOR(
        monitor_handle,
        ptr::addr_of_mut!(num_physical_monitors),
    )
    .unwrap();

    let mut physical_monitors =
        Vec::with_capacity(num_physical_monitors as usize);
    GetPhysicalMonitorsFromHMONITOR(
        monitor_handle,
        slice::from_raw_parts_mut(
            physical_monitors.as_mut_ptr(),
            num_physical_monitors as usize,
        ),
    )
    .unwrap();
    // SAFETY: The new length is equal to its capacity, and elements were
    // initialized by GetPhysicalMonitorsFromHMONITOR.
    physical_monitors.set_len(num_physical_monitors as usize);

    let monitor_handles = &mut *(data.0 as *mut Vec<HANDLE>);
    for monitor in physical_monitors {
        monitor_handles.push(monitor.hPhysicalMonitor);
    }

    // Return TRUE to continue the enumeration.
    TRUE
}

fn get_physical_monitor_handles() -> Vec<HANDLE> {
    let mut physical_monitor_handles: Vec<HANDLE> = Vec::new();
    unsafe {
        // Pass None, i.e., NULL, for the first two parameters to enumerate
        // all display monitors.
        EnumDisplayMonitors(
            None,
            None,
            Some(enum_display_monitors_callback),
            LPARAM(ptr::addr_of_mut!(physical_monitor_handles) as _),
        )
        .ok()
        .unwrap();
    }
    physical_monitor_handles
}

fn get_capabilities_string(handle: &HANDLE) -> Option<CString> {
    // TODO: Check capabilities functions' return values.
    unsafe {
        let mut capabilities_str_len: u32 = 0;
        GetCapabilitiesStringLength(
            *handle,
            ptr::addr_of_mut!(capabilities_str_len),
        );

        // TODO: Add retries for capabilities functions failures. I've seen
        // transient failures on my machine.
        if capabilities_str_len == 0 {
            return None;
        }

        let mut capabilities_str =
            Vec::with_capacity(capabilities_str_len as usize);
        CapabilitiesRequestAndCapabilitiesReply(
            *handle,
            slice::from_raw_parts_mut(
                capabilities_str.as_mut_ptr(),
                capabilities_str_len as usize,
            ),
        );
        // SAFETY: The new length is equal to its capacity, and the
        // elements were initialized by
        // CapabilitiesRequestAndCapabilitiesReply.
        capabilities_str.set_len(capabilities_str_len as usize);

        // Sometimes there's an extra nul byte (?). Find the first one and
        // truncate the rest.
        let nul_position =
            capabilities_str.iter().position(|&b| b == b'\0').unwrap();
        if nul_position < (capabilities_str.len() - 1) {
            capabilities_str.truncate(nul_position + 1);
        }

        CString::from_vec_with_nul(capabilities_str).ok()
    }
}

fn main() {
    let physical_monitor_handles = get_physical_monitor_handles();

    println!("Capabilities:");
    for handle in &physical_monitor_handles {
        if let Some(capabilities_string) = get_capabilities_string(handle) {
            println!("{:?}", capabilities_string);
            let _capabilities = parse::parse_capabilities_string(
                capabilities_string.to_str().unwrap(),
            );
        }
    }

    unsafe {
        for handle in physical_monitor_handles {
            DestroyPhysicalMonitor(handle).unwrap();
        }
    }
}
