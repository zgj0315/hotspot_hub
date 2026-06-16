#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrafficReading {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatteryReading {
    pub level_percent: Option<u8>,
    pub temperature_celsius: Option<f32>,
    pub is_charging: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetricAvailability<T> {
    Available(T),
    Unavailable { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricSample {
    pub timestamp_millis: u64,
    pub traffic: MetricAvailability<TrafficReading>,
    pub battery: MetricAvailability<BatteryReading>,
    pub connected_device_count: MetricAvailability<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionStatus {
    Stable,
    Attention,
    Risk,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeedSnapshot {
    pub down_bytes_per_second: u64,
    pub up_bytes_per_second: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DashboardState {
    pub status: SessionStatus,
    pub status_reason: String,
    pub session_duration_millis: u64,
    pub session_rx_bytes: Option<u64>,
    pub session_tx_bytes: Option<u64>,
    pub speed: Option<SpeedSnapshot>,
    pub connected_device_count: MetricAvailability<u32>,
    pub battery: MetricAvailability<BatteryReading>,
    pub speed_trend: Vec<SpeedSnapshot>,
    pub battery_trend: Vec<u8>,
    pub temperature_trend: Vec<f32>,
    pub last_updated_millis: Option<u64>,
}

impl DashboardState {
    pub fn initial() -> Self {
        Self {
            status: SessionStatus::Unknown,
            status_reason: "等待足够采样".into(),
            session_duration_millis: 0,
            session_rx_bytes: None,
            session_tx_bytes: None,
            speed: None,
            connected_device_count: MetricAvailability::Unavailable {
                reason: "等待足够采样".into(),
            },
            battery: MetricAvailability::Unavailable {
                reason: "等待足够采样".into(),
            },
            speed_trend: Vec::new(),
            battery_trend: Vec::new(),
            temperature_trend: Vec::new(),
            last_updated_millis: None,
        }
    }
}
