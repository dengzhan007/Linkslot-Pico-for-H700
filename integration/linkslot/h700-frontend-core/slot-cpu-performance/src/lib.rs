#![no_std]

//! Typed Rust client for the H700 CPU Performance Service C ABI v1.
//!
//! The C service is policy-only: a successful mode request selects a planned
//! OPP but does not apply a CPU clock or voltage transition. Actual telemetry
//! remains a separate optional set of read-back values.

use core::{
    ffi::c_void,
    marker::PhantomData,
    mem::size_of,
    ptr::NonNull,
};

pub const ABI_VERSION: u32 = 1;
pub const REQUEST_POLICY_ONLY: i32 = 1;

const TELEMETRY_CLOCK_VALID: u32 = 1 << 0;
const TELEMETRY_VOLTAGE_VALID: u32 = 1 << 1;
const TELEMETRY_TEMP_VALID: u32 = 1 << 2;
const TELEMETRY_VALID_MASK: u32 =
    TELEMETRY_CLOCK_VALID | TELEMETRY_VOLTAGE_VALID | TELEMETRY_TEMP_VALID;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuPerformanceMode {
    PowerSaver = 0,
    Balanced = 1,
    Performance = 2,
    Auto = 3,
    Boost = 4,
}

impl CpuPerformanceMode {
    fn from_raw(value: u32) -> Result<Self, CpuPerformanceError> {
        match value {
            0 => Ok(Self::PowerSaver),
            1 => Ok(Self::Balanced),
            2 => Ok(Self::Performance),
            3 => Ok(Self::Auto),
            4 => Ok(Self::Boost),
            other => Err(CpuPerformanceError::UnknownMode(other)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperatingPoint {
    pub frequency_hz: u32,
    pub voltage_uv: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuPerformanceSnapshot {
    pub generation: u32,
    pub requested_mode: CpuPerformanceMode,
    pub planned: Option<OperatingPoint>,
    pub actual_frequency_hz: Option<u32>,
    pub actual_voltage_uv: Option<u32>,
    pub temperature_mc: Option<i32>,
    pub policy_flags: u32,
    pub last_policy_status: i32,
    /// ABI v1 never applies hardware. A true value is treated as ABI corruption.
    pub hardware_applied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuPerformanceError {
    NullService,
    NotInitialized,
    InvalidArgument,
    AbiMismatch,
    NotReady,
    NoSafeOperatingPoint,
    ModeUnavailable,
    InvalidOppTable,
    InvalidState,
    UnknownMode(u32),
    UnknownStatus(i32),
}

impl CpuPerformanceError {
    fn from_c_status(status: i32) -> Self {
        match status {
            -1 | -100 => Self::InvalidArgument,
            -2 => Self::NotReady,
            -3 => Self::NoSafeOperatingPoint,
            -4 => Self::ModeUnavailable,
            -5 => Self::InvalidOppTable,
            -101 => Self::AbiMismatch,
            -102 => Self::NotInitialized,
            other => Self::UnknownStatus(other),
        }
    }
}

/// Exact ABI v1 state layout from h700_cpu_performance_service.h.
/// All fields are fixed-width so this layout is stable across AArch32/AArch64.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CpuPerformanceStateV1 {
    abi_version: u32,
    struct_size: u32,
    generation: u32,
    requested_mode: u32,
    planned_frequency_hz: u32,
    planned_voltage_uv: u32,
    policy_flags: u32,
    last_policy_status: i32,
    telemetry_valid_flags: u32,
    actual_frequency_hz: u32,
    actual_voltage_uv: u32,
    temperature_mc: i32,
    hardware_applied: u32,
    reserved: [u32; 3],
}

impl CpuPerformanceStateV1 {
    fn decode(self) -> Result<CpuPerformanceSnapshot, CpuPerformanceError> {
        if self.abi_version != ABI_VERSION ||
            self.struct_size as usize != size_of::<Self>() ||
            self.reserved != [0; 3] ||
            self.hardware_applied != 0 ||
            self.telemetry_valid_flags & !TELEMETRY_VALID_MASK != 0
        {
            return Err(CpuPerformanceError::AbiMismatch);
        }

        let planned = match (self.planned_frequency_hz, self.planned_voltage_uv) {
            (0, 0) => None,
            (frequency_hz, voltage_uv) if frequency_hz != 0 && voltage_uv != 0 => {
                Some(OperatingPoint { frequency_hz, voltage_uv })
            }
            _ => return Err(CpuPerformanceError::InvalidState),
        };

        let actual_frequency_hz =
            if self.telemetry_valid_flags & TELEMETRY_CLOCK_VALID != 0 {
                if self.actual_frequency_hz == 0 {
                    return Err(CpuPerformanceError::InvalidState);
                }
                Some(self.actual_frequency_hz)
            } else {
                None
            };

        let actual_voltage_uv =
            if self.telemetry_valid_flags & TELEMETRY_VOLTAGE_VALID != 0 {
                if self.actual_voltage_uv == 0 {
                    return Err(CpuPerformanceError::InvalidState);
                }
                Some(self.actual_voltage_uv)
            } else {
                None
            };

        let temperature_mc =
            if self.telemetry_valid_flags & TELEMETRY_TEMP_VALID != 0 {
                Some(self.temperature_mc)
            } else {
                None
            };

        Ok(CpuPerformanceSnapshot {
            generation: self.generation,
            requested_mode: CpuPerformanceMode::from_raw(self.requested_mode)?,
            planned,
            actual_frequency_hz,
            actual_voltage_uv,
            temperature_mc,
            policy_flags: self.policy_flags,
            last_policy_status: self.last_policy_status,
            hardware_applied: false,
        })
    }
}

extern "C" {
    fn h700_cpu_perf_service_request_mode_v1(
        service: *mut c_void,
        mode: i32,
    ) -> i32;

    fn h700_cpu_perf_service_get_state_v1(
        service: *const c_void,
        state: *mut CpuPerformanceStateV1,
        state_size: u32,
    ) -> i32;
}

/// Exclusive, borrowed access to a caller-owned C service instance.
///
/// # Safety
/// The pointer must refer to a live, initialized
/// `h700_cpu_perf_service_v1_t` for the full borrow. No other caller may
/// mutate that service while this client exists.
pub struct CpuPerformanceClient<'a> {
    service: NonNull<c_void>,
    _exclusive: PhantomData<&'a mut c_void>,
}

impl<'a> CpuPerformanceClient<'a> {
    /// Construct a client from a C-owned service instance.
    ///
    /// # Safety
    /// The caller must uphold the pointer validity and exclusive-access
    /// requirements documented on `CpuPerformanceClient`.
    pub unsafe fn from_raw(service: *mut c_void) -> Result<Self, CpuPerformanceError> {
        let service = NonNull::new(service).ok_or(CpuPerformanceError::NullService)?;
        Ok(Self {
            service,
            _exclusive: PhantomData,
        })
    }

    /// Request a policy mode. The only successful outcome in ABI v1 is
    /// `PolicyOnly`; it must never be presented as an applied clock change.
    pub fn request_mode(
        &mut self,
        mode: CpuPerformanceMode,
    ) -> Result<ModeRequest, CpuPerformanceError> {
        let status = unsafe {
            h700_cpu_perf_service_request_mode_v1(
                self.service.as_ptr(),
                mode as i32,
            )
        };
        if status != REQUEST_POLICY_ONLY {
            return Err(CpuPerformanceError::from_c_status(status));
        }
        Ok(ModeRequest::PolicyOnly(self.snapshot()?))
    }

    /// Read the last planned OPP and independent hardware telemetry.
    pub fn snapshot(&self) -> Result<CpuPerformanceSnapshot, CpuPerformanceError> {
        let mut raw = CpuPerformanceStateV1::default();
        let status = unsafe {
            h700_cpu_perf_service_get_state_v1(
                self.service.as_ptr(),
                &mut raw,
                size_of::<CpuPerformanceStateV1>() as u32,
            )
        };
        if status != 0 {
            return Err(CpuPerformanceError::from_c_status(status));
        }
        raw.decode()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeRequest {
    PolicyOnly(CpuPerformanceSnapshot),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> CpuPerformanceStateV1 {
        CpuPerformanceStateV1 {
            abi_version: ABI_VERSION,
            struct_size: size_of::<CpuPerformanceStateV1>() as u32,
            generation: 7,
            requested_mode: CpuPerformanceMode::Performance as u32,
            planned_frequency_hz: 1_400_000_000,
            planned_voltage_uv: 1_100_000,
            policy_flags: 0,
            last_policy_status: 0,
            telemetry_valid_flags: 0,
            actual_frequency_hz: 0,
            actual_voltage_uv: 0,
            temperature_mc: 0,
            hardware_applied: 0,
            reserved: [0; 3],
        }
    }

    #[test]
    fn c_state_layout_is_fixed_width() {
        assert_eq!(size_of::<CpuPerformanceStateV1>(), 64);
    }

    #[test]
    fn mode_ids_match_the_c_abi() {
        assert_eq!(CpuPerformanceMode::PowerSaver as u32, 0);
        assert_eq!(CpuPerformanceMode::Balanced as u32, 1);
        assert_eq!(CpuPerformanceMode::Performance as u32, 2);
        assert_eq!(CpuPerformanceMode::Auto as u32, 3);
        assert_eq!(CpuPerformanceMode::Boost as u32, 4);
    }

    #[test]
    fn planned_opp_is_not_reported_as_actual_frequency() {
        let decoded = state().decode().unwrap();
        assert_eq!(
            decoded.planned,
            Some(OperatingPoint {
                frequency_hz: 1_400_000_000,
                voltage_uv: 1_100_000,
            })
        );
        assert_eq!(decoded.actual_frequency_hz, None);
        assert_eq!(decoded.actual_voltage_uv, None);
        assert_eq!(decoded.temperature_mc, None);
        assert!(!decoded.hardware_applied);
    }

    #[test]
    fn actual_telemetry_is_kept_separate_from_the_plan() {
        let mut raw = state();
        raw.telemetry_valid_flags =
            TELEMETRY_CLOCK_VALID | TELEMETRY_VOLTAGE_VALID | TELEMETRY_TEMP_VALID;
        raw.actual_frequency_hz = 1_000_000_000;
        raw.actual_voltage_uv = 1_000_000;
        raw.temperature_mc = 62_500;
        let decoded = raw.decode().unwrap();

        assert_eq!(decoded.planned.unwrap().frequency_hz, 1_400_000_000);
        assert_eq!(decoded.actual_frequency_hz, Some(1_000_000_000));
        assert_eq!(decoded.actual_voltage_uv, Some(1_000_000));
        assert_eq!(decoded.temperature_mc, Some(62_500));
    }

    #[test]
    fn missing_or_malformed_telemetry_is_never_fabricated() {
        let mut raw = state();
        raw.telemetry_valid_flags = TELEMETRY_CLOCK_VALID;
        assert_eq!(raw.decode(), Err(CpuPerformanceError::InvalidState));

        raw.telemetry_valid_flags = 0;
        assert_eq!(raw.decode().unwrap().actual_frequency_hz, None);
    }

    #[test]
    fn unknown_mode_and_abi_fields_are_rejected() {
        let mut raw = state();
        raw.requested_mode = 99;
        assert_eq!(raw.decode(), Err(CpuPerformanceError::UnknownMode(99)));

        raw = state();
        raw.abi_version = 2;
        assert_eq!(raw.decode(), Err(CpuPerformanceError::AbiMismatch));

        raw = state();
        raw.reserved[1] = 1;
        assert_eq!(raw.decode(), Err(CpuPerformanceError::AbiMismatch));
    }

    #[test]
    fn unknown_telemetry_bits_and_hardware_applied_claim_are_rejected() {
        let mut raw = state();
        raw.telemetry_valid_flags = 1 << 31;
        assert_eq!(raw.decode(), Err(CpuPerformanceError::AbiMismatch));

        raw = state();
        raw.hardware_applied = 1;
        assert_eq!(raw.decode(), Err(CpuPerformanceError::AbiMismatch));
    }

    #[test]
    fn c_error_codes_are_mapped_without_becoming_success() {
        assert_eq!(CpuPerformanceError::from_c_status(-2), CpuPerformanceError::NotReady);
        assert_eq!(CpuPerformanceError::from_c_status(-4), CpuPerformanceError::ModeUnavailable);
        assert_eq!(CpuPerformanceError::from_c_status(-102), CpuPerformanceError::NotInitialized);
        assert_eq!(CpuPerformanceError::from_c_status(0), CpuPerformanceError::UnknownStatus(0));
    }
}
