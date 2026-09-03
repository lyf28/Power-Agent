use serde::Serialize;

#[cfg(target_os = "windows")]
use windows::core::PCWSTR;

#[cfg(target_os = "windows")]
use windows::Win32::System::Power::{
    GetSystemPowerStatus,
    SYSTEM_POWER_STATUS,
};

#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW,
    EnumDisplaySettingsExW,
    DISPLAY_DEVICEW,
    DISPLAY_DEVICE_PRIMARY_DEVICE,
    DEVMODEW,
    DM_BITSPERPEL,
    DM_DISPLAYFIXEDOUTPUT,
    DM_DISPLAYFLAGS,
    DM_DISPLAYFREQUENCY,
    DM_DISPLAYORIENTATION,
    DM_PELSHEIGHT,
    DM_PELSWIDTH,
    ENUM_DISPLAY_SETTINGS_FLAGS,
    ENUM_DISPLAY_SETTINGS_MODE,
    ENUM_CURRENT_SETTINGS,
};

#[derive(Serialize)]
struct BatteryStatus {
    percentage: u8,
    plugged_in: bool,
    charging: bool,
    remaining_seconds: Option<u32>,
}

#[derive(Serialize)]
struct DisplayMode {
    width: u32,
    height: u32,
    refresh_rate: u32,
    bits_per_pixel: u32,
    display_flags: u32,
    orientation: Option<u32>,
    fixed_output: Option<u32>,
    field_flags: u32,
}

#[derive(Serialize)]
struct DisplayInfo {
    display_device_name: String,
    current_mode: DisplayMode,
    driver_reported_mode_candidates: Vec<DisplayMode>,
}

#[tauri::command]
fn get_battery_status() -> Result<BatteryStatus, String> {
    unsafe {
        let mut status = SYSTEM_POWER_STATUS::default();

        GetSystemPowerStatus(&mut status)
            .map_err(|_| "Failed to get system power status".to_string())?;

        let percentage = status.BatteryLifePercent;

        let plugged_in = status.ACLineStatus == 1;

        let charging = status.BatteryFlag & 8 != 0;

        let remaining_seconds =
            if status.BatteryLifeTime == u32::MAX {
                None
            } else {
                Some(status.BatteryLifeTime)
            };

        Ok(BatteryStatus {
            percentage,
            plugged_in,
            charging,
            remaining_seconds,
        })
    }
}

#[tauri::command]
fn get_display_info() -> Result<DisplayInfo, String> {
    unsafe {
        let primary_device_name = get_primary_display_device_name()?;
        let display_device_name =
            null_terminated_utf16_to_string(&primary_device_name);
        let device_name = PCWSTR(primary_device_name.as_ptr());
        let mut current_mode = DEVMODEW::default();

        current_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;

        let result = EnumDisplaySettingsExW(
            device_name,
            ENUM_CURRENT_SETTINGS,
            &mut current_mode,
            ENUM_DISPLAY_SETTINGS_FLAGS::default(),
        );

        if !result.as_bool() {
            return Err(
                "Failed to get current display settings".to_string()
            );
        }

        if current_mode.dmDisplayFrequency <= 1 {
            return Err(
                "Windows did not report a concrete current refresh rate"
                    .to_string()
            );
        }

        let current_mode = display_mode_from_dev_mode(&current_mode)?;
        let mut driver_reported_mode_candidates = Vec::new();
        let mut mode_index = 0;

        loop {
            let mut candidate_mode = DEVMODEW::default();
            candidate_mode.dmSize =
                std::mem::size_of::<DEVMODEW>() as u16;

            let result = EnumDisplaySettingsExW(
                device_name,
                ENUM_DISPLAY_SETTINGS_MODE(mode_index),
                &mut candidate_mode,
                ENUM_DISPLAY_SETTINGS_FLAGS::default(),
            );

            if !result.as_bool() {
                break;
            }

            if candidate_mode.dmPelsWidth == current_mode.width
                && candidate_mode.dmPelsHeight == current_mode.height
                && candidate_mode.dmDisplayFrequency > 1
            {
                driver_reported_mode_candidates.push(
                    display_mode_from_dev_mode(&candidate_mode)?
                );
            }

            mode_index += 1;
        }

        if driver_reported_mode_candidates.is_empty() {
            return Err(
                "Failed to find driver-reported display mode candidates for the current resolution"
                    .to_string()
            );
        }

        Ok(DisplayInfo {
            display_device_name,
            current_mode,
            driver_reported_mode_candidates,
        })
    }
}

#[cfg(target_os = "windows")]
unsafe fn display_mode_from_dev_mode(
    dev_mode: &DEVMODEW,
) -> Result<DisplayMode, String> {
    let required_fields = DM_BITSPERPEL
        | DM_PELSWIDTH
        | DM_PELSHEIGHT
        | DM_DISPLAYFLAGS
        | DM_DISPLAYFREQUENCY;

    if !dev_mode.dmFields.contains(required_fields) {
        return Err(
            "Windows returned incomplete display mode information".to_string()
        );
    }

    let display_properties = dev_mode.Anonymous1.Anonymous2;

    let orientation = dev_mode
        .dmFields
        .contains(DM_DISPLAYORIENTATION)
        .then_some(display_properties.dmDisplayOrientation.0);

    let fixed_output = dev_mode
        .dmFields
        .contains(DM_DISPLAYFIXEDOUTPUT)
        .then_some(display_properties.dmDisplayFixedOutput.0);

    Ok(DisplayMode {
        width: dev_mode.dmPelsWidth,
        height: dev_mode.dmPelsHeight,
        refresh_rate: dev_mode.dmDisplayFrequency,
        bits_per_pixel: dev_mode.dmBitsPerPel,
        display_flags: dev_mode.Anonymous2.dmDisplayFlags,
        orientation,
        fixed_output,
        field_flags: dev_mode.dmFields.0,
    })
}

fn null_terminated_utf16_to_string(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(value.len());

    String::from_utf16_lossy(&value[..length])
}

#[cfg(target_os = "windows")]
unsafe fn get_primary_display_device_name() -> Result<[u16; 32], String> {
    let mut device_index = 0;

    loop {
        let mut display_device = DISPLAY_DEVICEW::default();
        display_device.cb =
            std::mem::size_of::<DISPLAY_DEVICEW>() as u32;

        let result = EnumDisplayDevicesW(
            None,
            device_index,
            &mut display_device,
            0,
        );

        if !result.as_bool() {
            break;
        }

        if display_device
            .StateFlags
            .contains(DISPLAY_DEVICE_PRIMARY_DEVICE)
        {
            return Ok(display_device.DeviceName);
        }

        device_index += 1;
    }

    Err("Failed to find the primary display device".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_battery_status,
            get_display_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
