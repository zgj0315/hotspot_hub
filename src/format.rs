use crate::model::SessionStatus;

pub fn format_duration(milliseconds: u64) -> String {
    let total_minutes = milliseconds / 60_000;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    if hours > 0 {
        format!("{hours} 小时 {minutes} 分钟")
    } else {
        format!("{minutes} 分钟")
    }
}

pub fn format_bytes(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "--".into();
    };

    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}

pub fn format_speed(bytes_per_second: Option<u64>) -> String {
    let Some(bytes_per_second) = bytes_per_second else {
        return "--".into();
    };
    let megabits = bytes_per_second as f64 * 8.0 / 1_000_000.0;
    format!("{megabits:.1} Mbps")
}

pub fn format_status(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Stable => "稳定",
        SessionStatus::Attention => "注意",
        SessionStatus::Risk => "风险",
        SessionStatus::Unknown => "未知",
    }
}

pub fn format_sample_count(label: &str, count: usize) -> String {
    format!("{label}样本：{count}")
}

pub fn format_last_updated(value: Option<u64>) -> &'static str {
    match value {
        Some(_) => "刚刚更新",
        None => "等待第一次采样",
    }
}
