use hotspot_hub::format::{format_bytes, format_duration, format_speed};

#[test]
fn formats_duration_as_hours_and_minutes() {
    assert_eq!(format_duration(8_280_000), "2h 18m");
}

#[test]
fn formats_bytes_as_gb() {
    assert_eq!(format_bytes(Some(1_800_000_000)), "1.8 GB");
}

#[test]
fn formats_missing_bytes_as_dash() {
    assert_eq!(format_bytes(None), "--");
}

#[test]
fn formats_speed_as_mbps() {
    assert_eq!(format_speed(Some(1_550_000)), "12.4 Mbps");
}
