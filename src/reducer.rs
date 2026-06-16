use crate::model::{
    DashboardState, MetricAvailability, MetricSample, SessionStatus, SpeedSnapshot,
};
use crate::ring_buffer::RingBuffer;

pub struct SessionReducer {
    baseline: Option<MetricSample>,
    previous: Option<MetricSample>,
    speed_trend: RingBuffer<SpeedSnapshot>,
    battery_trend: RingBuffer<u8>,
    temperature_trend: RingBuffer<f32>,
}

impl SessionReducer {
    pub fn new(trend_capacity: usize) -> Self {
        Self {
            baseline: None,
            previous: None,
            speed_trend: RingBuffer::new(trend_capacity),
            battery_trend: RingBuffer::new(trend_capacity),
            temperature_trend: RingBuffer::new(trend_capacity),
        }
    }

    pub fn accept(&mut self, sample: MetricSample) -> DashboardState {
        if self.baseline.is_none() && matches!(sample.traffic, MetricAvailability::Available(_)) {
            self.baseline = Some(sample.clone());
        }

        let speed = self.calculate_speed(&sample);
        if let Some(speed) = speed.clone() {
            self.speed_trend.push(speed);
        }

        if let MetricAvailability::Available(battery) = &sample.battery {
            if let Some(level) = battery.level_percent {
                self.battery_trend.push(level);
            }
            if let Some(temp) = battery.temperature_celsius {
                self.temperature_trend.push(temp);
            }
        }

        let (session_rx_bytes, session_tx_bytes) = self.session_traffic(&sample);
        let (status, status_reason) = self.classify(&sample, speed.as_ref());
        let start_time = self
            .baseline
            .as_ref()
            .map(|sample| sample.timestamp_millis)
            .unwrap_or(sample.timestamp_millis);

        self.previous = Some(sample.clone());

        DashboardState {
            status,
            status_reason,
            session_duration_millis: sample.timestamp_millis.saturating_sub(start_time),
            session_rx_bytes,
            session_tx_bytes,
            speed,
            connected_device_count: sample.connected_device_count,
            battery: sample.battery,
            speed_trend: self.speed_trend.to_vec(),
            battery_trend: self.battery_trend.to_vec(),
            temperature_trend: self.temperature_trend.to_vec(),
            last_updated_millis: Some(sample.timestamp_millis),
        }
    }

    fn calculate_speed(&self, current: &MetricSample) -> Option<SpeedSnapshot> {
        let previous = self.previous.as_ref()?;
        let MetricAvailability::Available(previous_traffic) = &previous.traffic else {
            return None;
        };
        let MetricAvailability::Available(current_traffic) = &current.traffic else {
            return None;
        };
        let elapsed = current
            .timestamp_millis
            .checked_sub(previous.timestamp_millis)?;
        if elapsed == 0 {
            return None;
        }

        Some(SpeedSnapshot {
            down_bytes_per_second: current_traffic
                .rx_bytes
                .saturating_sub(previous_traffic.rx_bytes)
                .saturating_mul(1_000)
                / elapsed,
            up_bytes_per_second: current_traffic
                .tx_bytes
                .saturating_sub(previous_traffic.tx_bytes)
                .saturating_mul(1_000)
                / elapsed,
        })
    }

    fn session_traffic(&self, current: &MetricSample) -> (Option<u64>, Option<u64>) {
        let Some(baseline) = &self.baseline else {
            return (None, None);
        };
        let MetricAvailability::Available(base_traffic) = &baseline.traffic else {
            return (None, None);
        };
        let MetricAvailability::Available(current_traffic) = &current.traffic else {
            return (None, None);
        };
        (
            Some(
                current_traffic
                    .rx_bytes
                    .saturating_sub(base_traffic.rx_bytes),
            ),
            Some(
                current_traffic
                    .tx_bytes
                    .saturating_sub(base_traffic.tx_bytes),
            ),
        )
    }

    fn classify(
        &self,
        sample: &MetricSample,
        speed: Option<&SpeedSnapshot>,
    ) -> (SessionStatus, String) {
        if let MetricAvailability::Unavailable { reason } = &sample.traffic {
            return (SessionStatus::Risk, localized_reason(reason).into());
        }

        let battery = match &sample.battery {
            MetricAvailability::Available(value) => value,
            MetricAvailability::Unavailable { reason } => {
                return (SessionStatus::Attention, localized_reason(reason).into());
            }
        };

        if battery.level_percent.is_some_and(|level| level < 10) && !battery.is_charging {
            return (SessionStatus::Risk, "电量严重偏低".into());
        }
        if battery
            .temperature_celsius
            .is_some_and(|temperature| temperature >= 45.0)
        {
            return (SessionStatus::Risk, "温度过高".into());
        }
        if battery.level_percent.is_some_and(|level| level < 20) {
            return (SessionStatus::Attention, "电量偏低".into());
        }
        if battery
            .temperature_celsius
            .is_some_and(|temperature| temperature >= 40.0)
        {
            return (SessionStatus::Attention, "温度偏高".into());
        }
        if speed.is_none() {
            return (SessionStatus::Unknown, "等待足够采样".into());
        }

        (SessionStatus::Stable, "热点会话状态稳定".into())
    }
}

fn localized_reason(reason: &str) -> &str {
    match reason {
        "Traffic counters unavailable on this device" => "此设备不允许读取流量计数",
        "Connected count restricted by system" => "系统限制读取连接设备数",
        "Waiting for enough samples" => "等待足够采样",
        "Battery state unavailable" => "无法读取电池状态",
        _ => reason,
    }
}
