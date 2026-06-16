use crate::model::{BatteryReading, MetricAvailability, TrafficReading};
use crate::sources::{BatterySource, ConnectedDeviceCountSource, TrafficSource};

pub struct DesktopTrafficSource {
    rx: u64,
    tx: u64,
}

impl DesktopTrafficSource {
    pub fn new() -> Self {
        Self {
            rx: 1_000_000,
            tx: 250_000,
        }
    }
}

impl TrafficSource for DesktopTrafficSource {
    fn read_traffic(&mut self) -> MetricAvailability<TrafficReading> {
        self.rx += 1_550_000;
        self.tx += 390_000;
        MetricAvailability::Available(TrafficReading {
            rx_bytes: self.rx,
            tx_bytes: self.tx,
        })
    }
}

pub struct DesktopBatterySource;

impl BatterySource for DesktopBatterySource {
    fn read_battery(&mut self) -> MetricAvailability<BatteryReading> {
        MetricAvailability::Available(BatteryReading {
            level_percent: Some(68),
            temperature_celsius: Some(39.0),
            is_charging: true,
        })
    }
}

pub struct DesktopConnectedDeviceCountSource;

impl ConnectedDeviceCountSource for DesktopConnectedDeviceCountSource {
    fn read_connected_device_count(&mut self) -> MetricAvailability<u32> {
        MetricAvailability::Available(4)
    }
}
