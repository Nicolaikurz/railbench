pub const APP_NAME: &str = "RAILbench";
#[allow(dead_code)]
pub const APP_VERSION: &str = env!("RAILBENCH_APP_VERSION");
pub const APP_VERSION_TAG: &str = env!("RAILBENCH_APP_VERSION_TAG");

pub fn full_app_title() -> String {
    format!("{APP_NAME} {APP_VERSION_TAG}")
}

#[allow(dead_code)]
pub fn format_calver(year: i32, month: u32, patch: u32) -> String {
    format!("{:02}.{:02}.{patch}", year.rem_euclid(100), month.clamp(1, 12))
}

#[allow(dead_code)]
pub fn format_calver_tag(year: i32, month: u32, patch: u32) -> String {
    format!("v{}", format_calver(year, month, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_version_follows_calver_yy_mm_patch_format() {
        let parts: Vec<&str> = APP_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "expected YY.MM.PATCH format, got {APP_VERSION}");
        assert_eq!(parts[0].len(), 2, "expected 2-digit year, got {}", parts[0]);
        assert_eq!(parts[1].len(), 2, "expected 2-digit month, got {}", parts[1]);

        let yy: u32 = parts[0].parse().expect("numeric year");
        let mm: u32 = parts[1].parse().expect("numeric month");
        let _patch: u32 = parts[2].parse().expect("numeric patch");

        assert!(yy >= 26, "expected build year >= 26, got {yy}");
        assert!((1..=12).contains(&mm), "expected build month in 1..=12, got {mm}");
        assert_eq!(APP_VERSION_TAG, format!("v{APP_VERSION}"));
        assert_eq!(full_app_title(), format!("RAILbench v{APP_VERSION}"));
    }

    #[test]
    fn calver_formatter_produces_expected_examples() {
        assert_eq!(format_calver_tag(2026, 12, 1), "v26.12.1");
        assert_eq!(format_calver_tag(2026, 11, 1), "v26.11.1");
        assert_eq!(format_calver_tag(2027, 1, 2), "v27.01.2");
    }
}
