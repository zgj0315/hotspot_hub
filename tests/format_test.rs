use hotspot_hub::format::{
    format_bytes, format_duration, format_last_updated, format_sample_count, format_speed,
    format_status,
};
use hotspot_hub::model::SessionStatus;

#[test]
fn formats_duration_as_hours_and_minutes() {
    assert_eq!(format_duration(8_280_000), "2 小时 18 分钟");
}

#[test]
fn formats_duration_as_minutes() {
    assert_eq!(format_duration(60_000), "1 分钟");
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

#[test]
fn formats_status_in_simplified_chinese() {
    assert_eq!(format_status(SessionStatus::Unknown), "未知");
    assert_eq!(format_status(SessionStatus::Stable), "稳定");
    assert_eq!(format_status(SessionStatus::Attention), "注意");
    assert_eq!(format_status(SessionStatus::Risk), "风险");
}

#[test]
fn formats_sample_count_in_simplified_chinese() {
    assert_eq!(format_sample_count("速度", 44), "速度样本：44");
}

#[test]
fn formats_last_updated_without_raw_timestamp() {
    assert_eq!(format_last_updated(Some(1781623527010)), "刚刚更新");
    assert_eq!(format_last_updated(None), "等待第一次采样");
}
