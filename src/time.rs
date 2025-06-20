#![allow(non_camel_case_types)]
use std::{
    ffi::{c_char, c_int, c_long},
    time::{Duration, SystemTime},
};

pub fn now_in_epoch_days() -> u16 {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let offset = timezone_offset(now.as_secs());
    let is_offset_positive = offset > 0;
    let offset = offset.unsigned_abs();

    let now = if is_offset_positive {
        now + Duration::from_secs(offset)
    } else {
        now - Duration::from_secs(offset)
    };

    now.div_duration_f32(Duration::from_secs(24 * 60 * 60)) as u16
}

fn timezone_offset(epoch_time_seconds: u64) -> i64 {
    let now_secs = i64::try_from(epoch_time_seconds).unwrap();
    let mut out = tm {
        tm_sec: 0,
        tm_min: 0,
        tm_hour: 0,
        tm_mday: 0,
        tm_mon: 0,
        tm_year: 0,
        tm_wday: 0,
        tm_yday: 0,
        tm_isdst: 0,
        tm_gmtoff: 0,
        tm_zone: std::ptr::null(),
    };

    let ret = unsafe { localtime_r(&now_secs, &mut out) };
    if ret.is_null() {
        panic!("failed to determine timezone offset for timestamp {epoch_time_seconds}");
    }

    out.tm_gmtoff
}

pub type time_t = i64;

#[repr(C)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: c_long,
    pub tm_zone: *const c_char,
}

unsafe extern "C" {
    pub fn localtime_r(time_p: *const time_t, result: *mut tm) -> *mut tm;
}
