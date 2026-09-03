use serde::Serialize;

#[cfg(target_os = "windows")]
use windows::core::PCWSTR;

#[cfg(target_os = "windows")]
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig, SetDisplayConfig,
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_MODE_INFO_TYPE,
    DISPLAYCONFIG_MODE_INFO_TYPE_DESKTOP_IMAGE, DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE,
    DISPLAYCONFIG_MODE_INFO_TYPE_TARGET, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_RATIONAL,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS, QDC_VIRTUAL_MODE_AWARE,
    QDC_VIRTUAL_REFRESH_RATE_AWARE, QUERY_DISPLAY_CONFIG_FLAGS, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
    SDC_VALIDATE, SDC_VIRTUAL_MODE_AWARE, SDC_VIRTUAL_REFRESH_RATE_AWARE, SET_DISPLAY_CONFIG_FLAGS,
};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{
    ERROR_BAD_CONFIGURATION, ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_PARAMETER, ERROR_SUCCESS,
    LUID,
};

#[cfg(target_os = "windows")]
use windows::Win32::System::Power::{
    GetSystemPowerStatus,
    SYSTEM_POWER_STATUS,
};

#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsExW, CDS_TEST, DEVMODEW,
    DISPLAYCONFIG_PATH_ACTIVE, DISPLAYCONFIG_PATH_CLONE_GROUP_INVALID,
    DISPLAYCONFIG_PATH_DESKTOP_IMAGE_IDX_INVALID, DISPLAYCONFIG_PATH_MODE_IDX_INVALID,
    DISPLAYCONFIG_PATH_SOURCE_MODE_IDX_INVALID, DISPLAYCONFIG_PATH_SUPPORT_VIRTUAL_MODE,
    DISPLAYCONFIG_PATH_TARGET_MODE_IDX_INVALID, DISPLAY_DEVICEW, DISPLAY_DEVICE_PRIMARY_DEVICE,
    DISP_CHANGE, DISP_CHANGE_BADDUALVIEW, DISP_CHANGE_BADFLAGS, DISP_CHANGE_BADMODE,
    DISP_CHANGE_BADPARAM, DISP_CHANGE_FAILED, DISP_CHANGE_NOTUPDATED, DISP_CHANGE_RESTART,
    DISP_CHANGE_SUCCESSFUL, DM_BITSPERPEL, DM_DISPLAYFIXEDOUTPUT, DM_DISPLAYFLAGS,
    DM_DISPLAYFREQUENCY, DM_DISPLAYORIENTATION, DM_PELSHEIGHT, DM_PELSWIDTH, DM_POSITION,
    ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_FLAGS, ENUM_DISPLAY_SETTINGS_MODE,
};

#[derive(Serialize)]
struct BatteryStatus {
    percentage: u8,
    plugged_in: bool,
    charging: bool,
    remaining_seconds: Option<u32>,
}

#[derive(Clone, Serialize)]
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

#[derive(Clone, Serialize)]
struct GdiEnumeratedCandidate {
    enumeration_index: u32,
    mode: DisplayMode,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum GdiCandidatePreflightStatus {
    Accepted,
    Rejected,
    UnavailableOrError,
    Ambiguous,
    NotFound,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum GdiPreflightEvidenceLevel {
    GdiDriverPreflight,
}

#[derive(Serialize)]
struct GdiCurrentModeSanityCheck {
    current_mode: DisplayMode,
    passed: bool,
    windows_result_code: Option<i32>,
    error: Option<String>,
}

#[derive(Serialize)]
struct GdiCandidatePreflight {
    requested_refresh_rate: u32,
    status: GdiCandidatePreflightStatus,
    windows_result_code: Option<i32>,
    display_device_name: Option<String>,
    matched_candidate_count: u32,
    candidate: Option<GdiEnumeratedCandidate>,
    matched_candidates: Vec<GdiEnumeratedCandidate>,
    current_mode_sanity_check: Option<GdiCurrentModeSanityCheck>,
    evidence_level: GdiPreflightEvidenceLevel,
    error: Option<String>,
}

impl GdiCandidatePreflight {
    fn new(requested_refresh_rate: u32) -> Self {
        Self {
            requested_refresh_rate,
            status: GdiCandidatePreflightStatus::UnavailableOrError,
            windows_result_code: None,
            display_device_name: None,
            matched_candidate_count: 0,
            candidate: None,
            matched_candidates: Vec::new(),
            current_mode_sanity_check: None,
            evidence_level: GdiPreflightEvidenceLevel::GdiDriverPreflight,
            error: None,
        }
    }

    fn unavailable(mut self, windows_result_code: Option<i32>, error: String) -> Self {
        self.status = GdiCandidatePreflightStatus::UnavailableOrError;
        self.windows_result_code = windows_result_code;
        self.error = Some(error);
        self
    }

    fn set_matched_candidates(&mut self, candidates: Vec<GdiEnumeratedCandidate>) {
        self.matched_candidate_count = candidates.len() as u32;
        self.candidate = (candidates.len() == 1)
            .then(|| candidates.first().cloned())
            .flatten();
        self.matched_candidates = candidates;
    }
}

#[cfg(target_os = "windows")]
struct NativeGdiCandidate {
    enumeration_index: u32,
    dev_mode: DEVMODEW,
}

#[derive(Serialize)]
struct AdapterLuid {
    low_part: u32,
    high_part: i32,
}

#[derive(Serialize)]
struct RefreshRateRational {
    numerator: u32,
    denominator: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum CcdModeType {
    Source,
    Target,
    DesktopImage,
}

#[derive(Serialize)]
struct CcdModeReference {
    mode_info_index: u32,
    mode_type: CcdModeType,
    adapter_luid: AdapterLuid,
    id: u32,
}

#[derive(Serialize)]
struct CcdActivePath {
    source_adapter_luid: AdapterLuid,
    target_adapter_luid: AdapterLuid,
    source_id: u32,
    target_id: u32,
    source_name: String,
    is_active: bool,
    target_available: bool,
    path_flags: u32,
    source_status_flags: u32,
    target_status_flags: u32,
    supports_virtual_mode: bool,
    clone_group_id: Option<u32>,
    path_refresh_rate: RefreshRateRational,
    target_mode_refresh_rate: Option<RefreshRateRational>,
    source_mode: Option<CcdModeReference>,
    target_mode: Option<CcdModeReference>,
    desktop_image_mode: Option<CcdModeReference>,
    output_technology: i32,
    rotation: i32,
    scaling: i32,
    scan_line_ordering: i32,
}

#[derive(Serialize)]
struct CcdDisplayMapping {
    query_flags: Option<u32>,
    active_paths: Vec<CcdActivePath>,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum CurrentCcdConfigurationValidationStatus {
    Valid,
    Invalid,
    UnavailableOrError,
}

#[derive(Serialize)]
struct CurrentCcdConfigurationValidation {
    status: CurrentCcdConfigurationValidationStatus,
    windows_result_code: Option<u32>,
    query_flags: Option<u32>,
    validation_flags: Option<u32>,
    path_count: u32,
    mode_count: u32,
    error: Option<String>,
}

#[derive(Serialize)]
struct DisplayInfo {
    display_device_name: String,
    current_mode: DisplayMode,
    driver_reported_mode_candidates: Vec<DisplayMode>,
    ccd_mapping: CcdDisplayMapping,
}

#[cfg(target_os = "windows")]
struct CcdSnapshot {
    query_flags: QUERY_DISPLAY_CONFIG_FLAGS,
    paths: Vec<DISPLAYCONFIG_PATH_INFO>,
    modes: Vec<DISPLAYCONFIG_MODE_INFO>,
}

#[cfg(target_os = "windows")]
enum CcdQueryError {
    WindowsApi { operation: &'static str, code: u32 },
    RetryLimit,
    InvalidData(String),
}

#[cfg(target_os = "windows")]
impl std::fmt::Display for CcdQueryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WindowsApi { operation, code } => {
                write!(formatter, "{operation} failed with Windows error {code}")
            }
            Self::RetryLimit => write!(
                formatter,
                "Display configuration kept changing while it was queried"
            ),
            Self::InvalidData(message) => formatter.write_str(message),
        }
    }
}

#[cfg(target_os = "windows")]
impl CcdQueryError {
    fn windows_result_code(&self) -> Option<u32> {
        match self {
            Self::WindowsApi { code, .. } => Some(*code),
            Self::RetryLimit => Some(ERROR_INSUFFICIENT_BUFFER.0),
            Self::InvalidData(_) => None,
        }
    }
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
fn validate_current_ccd_configuration() -> CurrentCcdConfigurationValidation {
    unsafe {
        let snapshot = match query_active_ccd_snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return CurrentCcdConfigurationValidation {
                    status: CurrentCcdConfigurationValidationStatus::UnavailableOrError,
                    windows_result_code: error.windows_result_code(),
                    query_flags: None,
                    validation_flags: None,
                    path_count: 0,
                    mode_count: 0,
                    error: Some(error.to_string()),
                };
            }
        };
        let validation_flags = current_ccd_validation_flags(snapshot.query_flags);
        let path_count = snapshot.paths.len() as u32;
        let mode_count = snapshot.modes.len() as u32;
        let windows_result = SetDisplayConfig(
            Some(&snapshot.paths),
            Some(&snapshot.modes),
            validation_flags,
        );
        let windows_result_code = windows_result as u32;

        let (status, error) = if windows_result == ERROR_SUCCESS.0 as i32 {
            (CurrentCcdConfigurationValidationStatus::Valid, None)
        } else if windows_result == ERROR_BAD_CONFIGURATION.0 as i32 {
            (
                CurrentCcdConfigurationValidationStatus::Invalid,
                Some("Windows rejected the supplied current CCD configuration".to_string()),
            )
        } else {
            (
                CurrentCcdConfigurationValidationStatus::UnavailableOrError,
                Some(format!(
                    "SetDisplayConfig(SDC_VALIDATE) failed with Windows error {windows_result_code}"
                )),
            )
        };

        CurrentCcdConfigurationValidation {
            status,
            windows_result_code: Some(windows_result_code),
            query_flags: Some(snapshot.query_flags.0),
            validation_flags: Some(validation_flags.0),
            path_count,
            mode_count,
            error,
        }
    }
}

#[tauri::command]
fn preflight_gdi_refresh_rate_candidate(requested_refresh_rate: u32) -> GdiCandidatePreflight {
    unsafe { preflight_gdi_refresh_rate_candidate_impl(requested_refresh_rate) }
}

#[cfg(target_os = "windows")]
unsafe fn preflight_gdi_refresh_rate_candidate_impl(
    requested_refresh_rate: u32,
) -> GdiCandidatePreflight {
    let mut preflight = GdiCandidatePreflight::new(requested_refresh_rate);

    if requested_refresh_rate <= 1 {
        return preflight.unavailable(
            None,
            "Requested refresh rate must be greater than 1 Hz".to_string(),
        );
    }

    let primary_device_name = match get_primary_display_device_name() {
        Ok(device_name) => device_name,
        Err(error) => return preflight.unavailable(None, error),
    };
    preflight.display_device_name = Some(null_terminated_utf16_to_string(&primary_device_name));
    let device_name = PCWSTR(primary_device_name.as_ptr());
    let current_native_mode = match get_current_native_gdi_mode(device_name) {
        Ok(mode) => mode,
        Err(error) => return preflight.unavailable(None, error),
    };
    let current_mode = match display_mode_from_dev_mode(&current_native_mode) {
        Ok(mode) => mode,
        Err(error) => return preflight.unavailable(None, error),
    };
    let native_candidates = match enumerate_matching_native_gdi_candidates(
        device_name,
        &current_native_mode,
        requested_refresh_rate,
    ) {
        Ok(candidates) => candidates,
        Err(error) => return preflight.unavailable(None, error),
    };
    let matched_candidates = match summarize_native_gdi_candidates(&native_candidates) {
        Ok(candidates) => candidates,
        Err(error) => return preflight.unavailable(None, error),
    };
    preflight.set_matched_candidates(matched_candidates);

    let current_sanity_candidates = match enumerate_matching_native_gdi_candidates(
        device_name,
        &current_native_mode,
        current_native_mode.dmDisplayFrequency,
    ) {
        Ok(candidates) => candidates,
        Err(error) => {
            preflight.current_mode_sanity_check = Some(GdiCurrentModeSanityCheck {
                current_mode,
                passed: false,
                windows_result_code: None,
                error: Some(error.clone()),
            });
            return preflight.unavailable(None, error);
        }
    };

    if current_sanity_candidates.len() != 1 {
        let error = format!(
            "Expected exactly one enumerated native mode matching the current {} Hz configuration, but found {}; current-mode sanity was not attempted",
            current_native_mode.dmDisplayFrequency,
            current_sanity_candidates.len(),
        );
        preflight.current_mode_sanity_check = Some(GdiCurrentModeSanityCheck {
            current_mode,
            passed: false,
            windows_result_code: None,
            error: Some(error.clone()),
        });
        return preflight.unavailable(None, error);
    }

    let sanity_result = ChangeDisplaySettingsExW(
        device_name,
        Some(&raw const current_sanity_candidates[0].dev_mode),
        None,
        CDS_TEST,
        None,
    );
    let sanity_passed = sanity_result == DISP_CHANGE_SUCCESSFUL;
    preflight.current_mode_sanity_check = Some(GdiCurrentModeSanityCheck {
        current_mode,
        passed: sanity_passed,
        windows_result_code: Some(sanity_result.0),
        error: (!sanity_passed).then(|| {
            format!(
                "Current-mode CDS_TEST sanity check returned {} ({})",
                sanity_result.0,
                display_change_result_name(sanity_result),
            )
        }),
    });

    if !sanity_passed {
        return preflight.unavailable(
            Some(sanity_result.0),
            "The current display mode did not pass CDS_TEST; the validation environment or implementation is unsuitable, so the requested candidate was not classified"
                .to_string(),
        );
    }

    if native_candidates.is_empty() {
        preflight.status = GdiCandidatePreflightStatus::NotFound;
        preflight.error = Some(format!(
            "No refresh-only {requested_refresh_rate} Hz mode was reported for the primary display at the current resolution"
        ));
        return preflight;
    }

    if native_candidates.len() > 1 {
        preflight.status = GdiCandidatePreflightStatus::Ambiguous;
        preflight.error = Some(format!(
            "Multiple enumerated native modes matched the {requested_refresh_rate} Hz request; no candidate was selected"
        ));
        return preflight;
    }

    let candidate_result = ChangeDisplaySettingsExW(
        device_name,
        Some(&raw const native_candidates[0].dev_mode),
        None,
        CDS_TEST,
        None,
    );
    preflight.status = classify_candidate_display_change_result(candidate_result);
    preflight.windows_result_code = Some(candidate_result.0);
    preflight.error = (preflight.status != GdiCandidatePreflightStatus::Accepted).then(|| {
        format!(
            "Candidate CDS_TEST returned {} ({})",
            candidate_result.0,
            display_change_result_name(candidate_result),
        )
    });
    preflight
}

#[tauri::command]
fn get_display_info() -> Result<DisplayInfo, String> {
    unsafe {
        let primary_device_name = get_primary_display_device_name()?;
        let display_device_name =
            null_terminated_utf16_to_string(&primary_device_name);
        let device_name = PCWSTR(primary_device_name.as_ptr());
        let mut current_mode = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };

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
            let mut candidate_mode = DEVMODEW {
                dmSize: std::mem::size_of::<DEVMODEW>() as u16,
                ..Default::default()
            };

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

        let ccd_mapping = get_ccd_display_mapping(&display_device_name).unwrap_or_else(|error| {
            CcdDisplayMapping {
                query_flags: None,
                active_paths: Vec::new(),
                error: Some(error),
            }
        });

        Ok(DisplayInfo {
            display_device_name,
            current_mode,
            driver_reported_mode_candidates,
            ccd_mapping,
        })
    }
}

#[cfg(target_os = "windows")]
unsafe fn get_current_native_gdi_mode(device_name: PCWSTR) -> Result<DEVMODEW, String> {
    let mut current_mode = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };

    let result = EnumDisplaySettingsExW(
        device_name,
        ENUM_CURRENT_SETTINGS,
        &mut current_mode,
        ENUM_DISPLAY_SETTINGS_FLAGS::default(),
    );

    if !result.as_bool() {
        return Err("Failed to get current GDI display settings for preflight".to_string());
    }

    ensure_self_contained_native_dev_mode(&current_mode, "current GDI mode")?;

    if current_mode.dmDisplayFrequency <= 1 {
        return Err("Windows did not report a concrete current refresh rate".to_string());
    }

    Ok(current_mode)
}

#[cfg(target_os = "windows")]
unsafe fn enumerate_matching_native_gdi_candidates(
    device_name: PCWSTR,
    current_mode: &DEVMODEW,
    requested_refresh_rate: u32,
) -> Result<Vec<NativeGdiCandidate>, String> {
    let mut matching_candidates = Vec::new();
    let mut mode_index = 0_u32;

    loop {
        let mut candidate_mode = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };

        let result = EnumDisplaySettingsExW(
            device_name,
            ENUM_DISPLAY_SETTINGS_MODE(mode_index),
            &mut candidate_mode,
            ENUM_DISPLAY_SETTINGS_FLAGS::default(),
        );

        if !result.as_bool() {
            break;
        }

        if gdi_mode_matches_refresh_only_request(
            current_mode,
            &candidate_mode,
            requested_refresh_rate,
        ) {
            ensure_self_contained_native_dev_mode(&candidate_mode, "enumerated GDI candidate")?;
            matching_candidates.push(NativeGdiCandidate {
                enumeration_index: mode_index,
                dev_mode: candidate_mode,
            });
        }

        mode_index = mode_index
            .checked_add(1)
            .ok_or_else(|| "GDI display mode index overflowed".to_string())?;
    }

    Ok(matching_candidates)
}

#[cfg(target_os = "windows")]
unsafe fn gdi_mode_matches_refresh_only_request(
    current_mode: &DEVMODEW,
    candidate_mode: &DEVMODEW,
    requested_refresh_rate: u32,
) -> bool {
    let required_fields =
        DM_BITSPERPEL | DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFLAGS | DM_DISPLAYFREQUENCY;

    if !current_mode.dmFields.contains(required_fields)
        || !candidate_mode.dmFields.contains(required_fields)
    {
        return false;
    }

    if candidate_mode.dmPelsWidth != current_mode.dmPelsWidth
        || candidate_mode.dmPelsHeight != current_mode.dmPelsHeight
        || candidate_mode.dmDisplayFrequency != requested_refresh_rate
        || candidate_mode.dmBitsPerPel != current_mode.dmBitsPerPel
        || candidate_mode.Anonymous2.dmDisplayFlags != current_mode.Anonymous2.dmDisplayFlags
    {
        return false;
    }

    if !optional_dev_mode_field_matches(
        current_mode.dmFields.contains(DM_DISPLAYORIENTATION),
        candidate_mode.dmFields.contains(DM_DISPLAYORIENTATION),
        current_mode.Anonymous1.Anonymous2.dmDisplayOrientation.0,
        candidate_mode.Anonymous1.Anonymous2.dmDisplayOrientation.0,
    ) || !optional_dev_mode_field_matches(
        current_mode.dmFields.contains(DM_DISPLAYFIXEDOUTPUT),
        candidate_mode.dmFields.contains(DM_DISPLAYFIXEDOUTPUT),
        current_mode.Anonymous1.Anonymous2.dmDisplayFixedOutput.0,
        candidate_mode.Anonymous1.Anonymous2.dmDisplayFixedOutput.0,
    ) {
        return false;
    }

    let current_has_position = current_mode.dmFields.contains(DM_POSITION);
    let candidate_has_position = candidate_mode.dmFields.contains(DM_POSITION);

    optional_dev_mode_field_matches(
        current_has_position,
        candidate_has_position,
        current_mode.Anonymous1.Anonymous2.dmPosition.x,
        candidate_mode.Anonymous1.Anonymous2.dmPosition.x,
    ) && optional_dev_mode_field_matches(
        current_has_position,
        candidate_has_position,
        current_mode.Anonymous1.Anonymous2.dmPosition.y,
        candidate_mode.Anonymous1.Anonymous2.dmPosition.y,
    )
}

#[cfg(target_os = "windows")]
fn optional_dev_mode_field_matches<T: Eq>(
    current_is_valid: bool,
    candidate_is_valid: bool,
    current_value: T,
    candidate_value: T,
) -> bool {
    !candidate_is_valid || (current_is_valid && current_value == candidate_value)
}

#[cfg(target_os = "windows")]
fn ensure_self_contained_native_dev_mode(
    dev_mode: &DEVMODEW,
    description: &str,
) -> Result<(), String> {
    if dev_mode.dmDriverExtra == 0 {
        Ok(())
    } else {
        Err(format!(
            "The {description} reported {} bytes of driver-private DEVMODE data that were not captured; preflight was not attempted",
            dev_mode.dmDriverExtra,
        ))
    }
}

#[cfg(target_os = "windows")]
unsafe fn summarize_native_gdi_candidates(
    candidates: &[NativeGdiCandidate],
) -> Result<Vec<GdiEnumeratedCandidate>, String> {
    candidates
        .iter()
        .map(|candidate| {
            Ok(GdiEnumeratedCandidate {
                enumeration_index: candidate.enumeration_index,
                mode: display_mode_from_dev_mode(&candidate.dev_mode)?,
            })
        })
        .collect()
}

#[cfg(target_os = "windows")]
fn classify_candidate_display_change_result(result: DISP_CHANGE) -> GdiCandidatePreflightStatus {
    if result == DISP_CHANGE_SUCCESSFUL {
        GdiCandidatePreflightStatus::Accepted
    } else if result == DISP_CHANGE_BADMODE || result == DISP_CHANGE_FAILED {
        GdiCandidatePreflightStatus::Rejected
    } else {
        GdiCandidatePreflightStatus::UnavailableOrError
    }
}

#[cfg(target_os = "windows")]
fn display_change_result_name(result: DISP_CHANGE) -> &'static str {
    if result == DISP_CHANGE_SUCCESSFUL {
        "DISP_CHANGE_SUCCESSFUL"
    } else if result == DISP_CHANGE_BADDUALVIEW {
        "DISP_CHANGE_BADDUALVIEW"
    } else if result == DISP_CHANGE_BADFLAGS {
        "DISP_CHANGE_BADFLAGS"
    } else if result == DISP_CHANGE_BADMODE {
        "DISP_CHANGE_BADMODE"
    } else if result == DISP_CHANGE_BADPARAM {
        "DISP_CHANGE_BADPARAM"
    } else if result == DISP_CHANGE_FAILED {
        "DISP_CHANGE_FAILED"
    } else if result == DISP_CHANGE_NOTUPDATED {
        "DISP_CHANGE_NOTUPDATED"
    } else if result == DISP_CHANGE_RESTART {
        "DISP_CHANGE_RESTART"
    } else {
        "UNKNOWN_DISP_CHANGE_RESULT"
    }
}

#[cfg(target_os = "windows")]
unsafe fn get_ccd_display_mapping(
    primary_gdi_device_name: &str,
) -> Result<CcdDisplayMapping, String> {
    let snapshot = query_active_ccd_snapshot().map_err(|error| error.to_string())?;
    let mut active_paths = Vec::new();

    for path in &snapshot.paths {
        let source_name = get_ccd_source_name(path).map_err(|error| error.to_string())?;

        if source_name.eq_ignore_ascii_case(primary_gdi_device_name) {
            active_paths.push(
                ccd_active_path_from_snapshot(path, &source_name, &snapshot.modes)
                    .map_err(|error| error.to_string())?,
            );
        }
    }

    if active_paths.is_empty() {
        return Err(format!(
            "No active CCD path matched primary GDI display {primary_gdi_device_name}"
        ));
    }

    Ok(CcdDisplayMapping {
        query_flags: Some(snapshot.query_flags.0),
        active_paths,
        error: None,
    })
}

#[cfg(target_os = "windows")]
unsafe fn query_active_ccd_snapshot() -> Result<CcdSnapshot, CcdQueryError> {
    let query_flag_options = [
        QUERY_DISPLAY_CONFIG_FLAGS(
            QDC_ONLY_ACTIVE_PATHS.0 | QDC_VIRTUAL_MODE_AWARE.0 | QDC_VIRTUAL_REFRESH_RATE_AWARE.0,
        ),
        QUERY_DISPLAY_CONFIG_FLAGS(QDC_ONLY_ACTIVE_PATHS.0 | QDC_VIRTUAL_MODE_AWARE.0),
        QDC_ONLY_ACTIVE_PATHS,
    ];

    let mut last_error = None;

    for query_flags in query_flag_options {
        match query_ccd_snapshot_with_flags(query_flags) {
            Ok(snapshot) => return Ok(snapshot),
            Err(error @ CcdQueryError::WindowsApi { code, .. })
                if code == ERROR_INVALID_PARAMETER.0 =>
            {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        CcdQueryError::InvalidData(
            "No compatible CCD query flag combination was available".to_string(),
        )
    }))
}

#[cfg(target_os = "windows")]
fn current_ccd_validation_flags(
    query_flags: QUERY_DISPLAY_CONFIG_FLAGS,
) -> SET_DISPLAY_CONFIG_FLAGS {
    let mut validation_flags = SDC_VALIDATE.0 | SDC_USE_SUPPLIED_DISPLAY_CONFIG.0;

    if query_flags.0 & QDC_VIRTUAL_MODE_AWARE.0 != 0 {
        validation_flags |= SDC_VIRTUAL_MODE_AWARE.0;
    }

    if query_flags.0 & QDC_VIRTUAL_REFRESH_RATE_AWARE.0 != 0 {
        validation_flags |= SDC_VIRTUAL_REFRESH_RATE_AWARE.0;
    }

    SET_DISPLAY_CONFIG_FLAGS(validation_flags)
}

#[cfg(target_os = "windows")]
unsafe fn query_ccd_snapshot_with_flags(
    query_flags: QUERY_DISPLAY_CONFIG_FLAGS,
) -> Result<CcdSnapshot, CcdQueryError> {
    const MAX_QUERY_ATTEMPTS: usize = 5;

    for _ in 0..MAX_QUERY_ATTEMPTS {
        let mut path_count = 0;
        let mut mode_count = 0;
        let result = GetDisplayConfigBufferSizes(query_flags, &mut path_count, &mut mode_count);

        if result != ERROR_SUCCESS {
            return Err(CcdQueryError::WindowsApi {
                operation: "GetDisplayConfigBufferSizes",
                code: result.0,
            });
        }

        if path_count == 0 || mode_count == 0 {
            return Err(CcdQueryError::InvalidData(
                "Windows returned an empty active display configuration".to_string(),
            ));
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        let result = QueryDisplayConfig(
            query_flags,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        );

        if result == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }

        if result != ERROR_SUCCESS {
            return Err(CcdQueryError::WindowsApi {
                operation: "QueryDisplayConfig",
                code: result.0,
            });
        }

        paths.truncate(path_count as usize);
        modes.truncate(mode_count as usize);

        return Ok(CcdSnapshot {
            query_flags,
            paths,
            modes,
        });
    }

    Err(CcdQueryError::RetryLimit)
}

#[cfg(target_os = "windows")]
unsafe fn get_ccd_source_name(path: &DISPLAYCONFIG_PATH_INFO) -> Result<String, CcdQueryError> {
    let mut source_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
            adapterId: path.sourceInfo.adapterId,
            id: path.sourceInfo.id,
        },
        ..Default::default()
    };

    let result = DisplayConfigGetDeviceInfo(&mut source_name.header);

    if result != ERROR_SUCCESS.0 as i32 {
        return Err(CcdQueryError::WindowsApi {
            operation: "DisplayConfigGetDeviceInfo(GET_SOURCE_NAME)",
            code: result as u32,
        });
    }

    Ok(null_terminated_utf16_to_string(
        &source_name.viewGdiDeviceName,
    ))
}

#[cfg(target_os = "windows")]
unsafe fn ccd_active_path_from_snapshot(
    path: &DISPLAYCONFIG_PATH_INFO,
    source_name: &str,
    modes: &[DISPLAYCONFIG_MODE_INFO],
) -> Result<CcdActivePath, CcdQueryError> {
    let supports_virtual_mode = path.flags & DISPLAYCONFIG_PATH_SUPPORT_VIRTUAL_MODE != 0;
    let raw_source_mode_info = path.sourceInfo.Anonymous.modeInfoIdx;
    let raw_target_mode_info = path.targetInfo.Anonymous.modeInfoIdx;

    let (source_mode_index, target_mode_index, desktop_image_mode_index, clone_group_id) =
        if supports_virtual_mode {
            let clone_group_id = raw_source_mode_info & 0xffff;
            let source_mode_index = (raw_source_mode_info >> 16) & 0xffff;
            let desktop_image_mode_index = raw_target_mode_info & 0xffff;
            let target_mode_index = (raw_target_mode_info >> 16) & 0xffff;

            (
                valid_mode_index(
                    source_mode_index,
                    DISPLAYCONFIG_PATH_SOURCE_MODE_IDX_INVALID,
                ),
                valid_mode_index(
                    target_mode_index,
                    DISPLAYCONFIG_PATH_TARGET_MODE_IDX_INVALID,
                ),
                valid_mode_index(
                    desktop_image_mode_index,
                    DISPLAYCONFIG_PATH_DESKTOP_IMAGE_IDX_INVALID,
                ),
                valid_mode_index(clone_group_id, DISPLAYCONFIG_PATH_CLONE_GROUP_INVALID),
            )
        } else {
            (
                valid_mode_index(raw_source_mode_info, DISPLAYCONFIG_PATH_MODE_IDX_INVALID),
                valid_mode_index(raw_target_mode_info, DISPLAYCONFIG_PATH_MODE_IDX_INVALID),
                None,
                None,
            )
        };

    let source_mode = mode_reference(
        modes,
        source_mode_index,
        DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE,
        CcdModeType::Source,
    )?;
    let target_mode = mode_reference(
        modes,
        target_mode_index,
        DISPLAYCONFIG_MODE_INFO_TYPE_TARGET,
        CcdModeType::Target,
    )?;
    let desktop_image_mode = mode_reference(
        modes,
        desktop_image_mode_index,
        DISPLAYCONFIG_MODE_INFO_TYPE_DESKTOP_IMAGE,
        CcdModeType::DesktopImage,
    )?;
    let target_mode_refresh_rate = target_mode_index
        .and_then(|index| modes.get(index as usize))
        .map(|mode_info| {
            refresh_rate_rational(
                mode_info
                    .Anonymous
                    .targetMode
                    .targetVideoSignalInfo
                    .vSyncFreq,
            )
        });

    Ok(CcdActivePath {
        source_adapter_luid: adapter_luid(path.sourceInfo.adapterId),
        target_adapter_luid: adapter_luid(path.targetInfo.adapterId),
        source_id: path.sourceInfo.id,
        target_id: path.targetInfo.id,
        source_name: source_name.to_string(),
        is_active: path.flags & DISPLAYCONFIG_PATH_ACTIVE != 0,
        target_available: path.targetInfo.targetAvailable.as_bool(),
        path_flags: path.flags,
        source_status_flags: path.sourceInfo.statusFlags,
        target_status_flags: path.targetInfo.statusFlags,
        supports_virtual_mode,
        clone_group_id,
        path_refresh_rate: refresh_rate_rational(path.targetInfo.refreshRate),
        target_mode_refresh_rate,
        source_mode,
        target_mode,
        desktop_image_mode,
        output_technology: path.targetInfo.outputTechnology.0,
        rotation: path.targetInfo.rotation.0,
        scaling: path.targetInfo.scaling.0,
        scan_line_ordering: path.targetInfo.scanLineOrdering.0,
    })
}

#[cfg(target_os = "windows")]
fn valid_mode_index(index: u32, invalid_value: u32) -> Option<u32> {
    (index != invalid_value).then_some(index)
}

#[cfg(target_os = "windows")]
fn mode_reference(
    modes: &[DISPLAYCONFIG_MODE_INFO],
    mode_info_index: Option<u32>,
    expected_type: DISPLAYCONFIG_MODE_INFO_TYPE,
    mode_type: CcdModeType,
) -> Result<Option<CcdModeReference>, CcdQueryError> {
    let Some(mode_info_index) = mode_info_index else {
        return Ok(None);
    };

    let mode_info = modes.get(mode_info_index as usize).ok_or_else(|| {
        CcdQueryError::InvalidData(format!(
            "CCD mode index {mode_info_index} was outside the returned mode table"
        ))
    })?;

    if mode_info.infoType != expected_type {
        return Err(CcdQueryError::InvalidData(format!(
            "CCD mode index {mode_info_index} had unexpected mode type {}",
            mode_info.infoType.0,
        )));
    }

    Ok(Some(CcdModeReference {
        mode_info_index,
        mode_type,
        adapter_luid: adapter_luid(mode_info.adapterId),
        id: mode_info.id,
    }))
}

#[cfg(target_os = "windows")]
fn adapter_luid(value: LUID) -> AdapterLuid {
    AdapterLuid {
        low_part: value.LowPart,
        high_part: value.HighPart,
    }
}

#[cfg(target_os = "windows")]
fn refresh_rate_rational(value: DISPLAYCONFIG_RATIONAL) -> RefreshRateRational {
    RefreshRateRational {
        numerator: value.Numerator,
        denominator: value.Denominator,
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

    let orientation = if dev_mode.dmFields.contains(DM_DISPLAYORIENTATION) {
        Some(dev_mode.Anonymous1.Anonymous2.dmDisplayOrientation.0)
    } else {
        None
    };

    let fixed_output = if dev_mode.dmFields.contains(DM_DISPLAYFIXEDOUTPUT) {
        Some(dev_mode.Anonymous1.Anonymous2.dmDisplayFixedOutput.0)
    } else {
        None
    };

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
        let mut display_device = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };

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

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn classifies_only_candidate_specific_driver_rejections_as_rejected() {
        assert_eq!(
            classify_candidate_display_change_result(DISP_CHANGE_SUCCESSFUL),
            GdiCandidatePreflightStatus::Accepted,
        );
        assert_eq!(
            classify_candidate_display_change_result(DISP_CHANGE_BADMODE),
            GdiCandidatePreflightStatus::Rejected,
        );
        assert_eq!(
            classify_candidate_display_change_result(DISP_CHANGE_FAILED),
            GdiCandidatePreflightStatus::Rejected,
        );
        assert_eq!(
            classify_candidate_display_change_result(DISP_CHANGE_BADPARAM),
            GdiCandidatePreflightStatus::UnavailableOrError,
        );
        assert_eq!(
            classify_candidate_display_change_result(DISP_CHANGE_BADDUALVIEW),
            GdiCandidatePreflightStatus::UnavailableOrError,
        );
        assert_eq!(
            classify_candidate_display_change_result(DISP_CHANGE_BADFLAGS),
            GdiCandidatePreflightStatus::UnavailableOrError,
        );
        assert_eq!(
            classify_candidate_display_change_result(DISP_CHANGE_NOTUPDATED),
            GdiCandidatePreflightStatus::UnavailableOrError,
        );
        assert_eq!(
            classify_candidate_display_change_result(DISP_CHANGE_RESTART),
            GdiCandidatePreflightStatus::UnavailableOrError,
        );
    }

    #[test]
    fn optional_candidate_fields_must_not_request_a_different_current_value() {
        assert!(optional_dev_mode_field_matches(true, true, 1, 1));
        assert!(optional_dev_mode_field_matches(false, false, 1, 2));
        assert!(!optional_dev_mode_field_matches(true, true, 1, 2));
        assert!(optional_dev_mode_field_matches(true, false, 1, 2));
        assert!(!optional_dev_mode_field_matches(false, true, 1, 1));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_battery_status,
            get_display_info,
            validate_current_ccd_configuration,
            preflight_gdi_refresh_rate_candidate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
