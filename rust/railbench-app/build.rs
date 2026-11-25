use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

/// Converts days since Unix epoch (1970-01-01) to `(year, month, day)` in the proleptic Gregorian calendar.
/// Algorithm by Howard Hinnant (`civil_from_days`).
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m as u32, d as u32)
}

fn current_utc_year_month() -> (i32, u32) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let (year, month, _day) = civil_from_days(days);
    (year, month)
}

fn main() {
    let version = if let Some(explicit) = env::var("RAILBENCH_VERSION")
        .ok()
        .map(|s| s.trim().trim_start_matches(['v', 'V']).to_string())
        .filter(|s| !s.is_empty())
    {
        explicit
    } else {
        let (year, month) = current_utc_year_month();
        let yy = year.rem_euclid(100);
        let patch = env::var("RAILBENCH_PATCH")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                env::var("CARGO_PKG_VERSION_PATCH").unwrap_or_else(|_| "1".to_string())
            });
        format!("{yy:02}.{month:02}.{patch}")
    };

    println!("cargo:rustc-env=RAILBENCH_APP_VERSION={version}");
    println!("cargo:rustc-env=RAILBENCH_APP_VERSION_TAG=v{version}");
    println!("cargo:rerun-if-env-changed=RAILBENCH_VERSION");
    println!("cargo:rerun-if-env-changed=RAILBENCH_PATCH");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=../Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");
}
