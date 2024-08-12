mod cap;
mod monitor;
mod parse;

use std::{
    ffi::{CStr, CString, OsStr, OsString},
    mem,
    os::windows::ffi::OsStringExt,
    ptr, slice,
};

use argh::FromArgs;
use windows::{
    core::PCSTR,
    Win32::{
        Devices::Display::{
            CapabilitiesRequestAndCapabilitiesReply, DestroyPhysicalMonitor,
            DisplayConfigGetDeviceInfo, GetCapabilitiesStringLength,
            GetDisplayConfigBufferSizes,
            GetNumberOfPhysicalMonitorsFromHMONITOR,
            GetPhysicalMonitorsFromHMONITOR, QueryDisplayConfig,
            DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            DISPLAYCONFIG_TARGET_DEVICE_NAME,
        },
        Foundation::{BOOL, ERROR_SUCCESS, HANDLE, LPARAM, RECT, TRUE},
        Graphics::Gdi::{
            EnumDisplayDevicesA, EnumDisplayMonitors, DISPLAY_DEVICEA, HDC,
            HMONITOR,
        },
    },
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

unsafe fn print_monitor_friendly_names() {
    let mut num_paths = 0;
    let mut num_modes = 0;
    GetDisplayConfigBufferSizes(
        windows::Win32::Devices::Display::QDC_ONLY_ACTIVE_PATHS,
        ptr::addr_of_mut!(num_paths),
        ptr::addr_of_mut!(num_modes),
    )
    .ok()
    .expect("GetDisplayConfigBufferSizes failed");

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
    .expect("QueryDisplayConfig failed");

    paths.set_len(num_paths as usize);
    modes.set_len(num_modes as usize);

    for path in paths {
        let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
        target.header.adapterId = path.targetInfo.adapterId;
        target.header.id = path.targetInfo.id;
        target.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
        target.header.size = mem::size_of_val(&target) as u32;

        if DisplayConfigGetDeviceInfo(ptr::addr_of_mut!(target.header)) == 0 {
            let len = target
                .monitorFriendlyDeviceName
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(0);
            let friendly_name =
                OsString::from_wide(&target.monitorFriendlyDeviceName[..len]);

            let len = target
                .monitorDevicePath
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(0);
            // The device path is the same as the device ID in DISPLAY_DEVICEA. So, I can associate the friendly name
            // to the monitors via the device ID
            let device_path =
                OsString::from_wide(&target.monitorDevicePath[..len]);

            println!("friendly name: {:?}", friendly_name);
            println!("device path: {:?}", device_path);
        } else {
            println!("DisplayConfigGetDeviceInfo failed");
        }
    }
}

unsafe fn print_display_devices() {
    let mut display_device_index = 0;

    let mut display_device = DISPLAY_DEVICEA::default();
    display_device.cb = mem::size_of::<DISPLAY_DEVICEA>() as u32;

    // CStr::from_ptr(display_device.DeviceName.as_ptr());

    while EnumDisplayDevicesA(
        None,
        display_device_index,
        ptr::addr_of_mut!(display_device),
        0,
    )
    .as_bool()
    {
        // Copy the device name since we reuse the display device struct for the next call.
        let device_name = display_device.DeviceName.clone();
        println!("device name: {:?}", CStr::from_ptr(device_name.as_ptr()));

        if EnumDisplayDevicesA(
            PCSTR::from_raw(device_name.as_ptr() as *const u8),
            0,
            ptr::addr_of_mut!(display_device),
            1,
        )
        .as_bool()
        {
            println!(
                "device string: {:?}",
                CStr::from_ptr(display_device.DeviceString.as_ptr())
            );
            println!(
                "device name: {:?}",
                CStr::from_ptr(display_device.DeviceName.as_ptr())
            );
            println!(
                "device ID: {:?}",
                CStr::from_ptr(display_device.DeviceID.as_ptr())
            );
            println!();
        }

        display_device_index += 1;
    }
}

// Iterate through monitors with EnumDisplayMonitors.
// For each monitor, get the device name with GetMonitorInfo
// Use the device name with EnumDisplayDevicesA to get the device ID

// TODO: Add an option to just try the window the terminal is on via MonitorFromWindow.
#[derive(FromArgs)]
#[argh(description = "chmi - change monitor input")]
struct Args {
    #[argh(switch, short = 'v', description = "use verbose output")]
    _verbose: bool,

    #[argh(switch, description = "print version information")]
    version: bool,
}

fn main() {
    let args: Args = argh::from_env();

    if args.version {
        // TODO: Print the commit hash too.
        println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }

    let _monitors = monitor::get_monitors().unwrap();

    // unsafe {
    //     print_display_devices();
    //     print_monitor_friendly_names();
    // }
    // return;

    // let physical_monitor_handles = get_physical_monitor_handles();

    // println!("Capabilities:");
    // for handle in &physical_monitor_handles {
    //     if let Some(capabilities_string) = get_capabilities_string(handle) {
    //         println!("{:?}", capabilities_string);
    //         let _capabilities = parse::parse_capabilities_string(
    //             capabilities_string.to_str().unwrap(),
    //         );
    //     }
    // }

    // // let monitors = get_monitors();
    // // check if monitors support change input via their capabilities
    // // print the ones that support it
    // // then print the available inputs of the selected monitor
    // // set the new value for the vcp code

    // unsafe {
    //     for handle in physical_monitor_handles {
    //         DestroyPhysicalMonitor(handle).unwrap();
    //     }
    // }
}
