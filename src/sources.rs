use crate::model::{BatteryReading, MetricAvailability, TrafficReading};

pub trait Clock {
    fn now_millis(&self) -> u64;
}

pub trait TrafficSource {
    fn read_traffic(&mut self) -> MetricAvailability<TrafficReading>;
}

pub trait BatterySource {
    fn read_battery(&mut self) -> MetricAvailability<BatteryReading>;
}

pub trait ConnectedDeviceCountSource {
    fn read_connected_device_count(&mut self) -> MetricAvailability<u32>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}
