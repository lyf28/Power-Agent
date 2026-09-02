use serde::Serialize;

#[cfg(target_os = "windows")]
use windows::Win32::System::Power::{
    GetSystemPowerStatus,
    SYSTEM_POWER_STATUS,
};

#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    EnumDisplaySettingsW,
    DEVMODEW,
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
struct DisplayInfo {
    refresh_rate: u32,
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
        let mut dev_mode = DEVMODEW::default();

        dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;

        let result = EnumDisplaySettingsW(
            None,
            ENUM_CURRENT_SETTINGS,
            &mut dev_mode,
        );

        if !result.as_bool() {
            return Err(
                "Failed to get current display settings".to_string()
            );
        }

        Ok(DisplayInfo {
            refresh_rate: dev_mode.dmDisplayFrequency,
        })
    }
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