use hotspot_hub::model::{
    BatteryReading, MetricAvailability, MetricSample, SessionStatus, TrafficReading,
};
use hotspot_hub::reducer::SessionReducer;

fn baseline() -> MetricSample {
    MetricSample {
        timestamp_millis: 1_000,
        traffic: MetricAvailability::Available(TrafficReading {
            rx_bytes: 1_000,
            tx_bytes: 500,
        }),
        battery: MetricAvailability::Available(BatteryReading {
            level_percent: Some(80),
            temperature_celsius: Some(35.0),
            is_charging: true,
        }),
        connected_device_count: MetricAvailability::Available(2),
    }
}

#[test]
fn computes_duration_traffic_and_speed_from_samples() {
    let mut reducer = SessionReducer::new(5);
    reducer.accept(baseline());
    let mut second = baseline();
    second.timestamp_millis = 2_000;
    second.traffic = MetricAvailability::Available(TrafficReading {
        rx_bytes: 3_000,
        tx_bytes: 1_500,
    });

    let state = reducer.accept(second);

    assert_eq!(state.session_duration_millis, 1_000);
    assert_eq!(state.session_rx_bytes, Some(2_000));
    assert_eq!(state.session_tx_bytes, Some(1_000));
    assert_eq!(state.speed.unwrap().down_bytes_per_second, 2_000);
    assert_eq!(state.status, SessionStatus::Stable);
}

#[test]
fn marks_unavailable_traffic_as_risk() {
    let mut reducer = SessionReducer::new(5);
    let mut sample = baseline();
    sample.traffic = MetricAvailability::Unavailable {
        reason: "Traffic counters unavailable on this device".into(),
    };

    let state = reducer.accept(sample);

    assert_eq!(state.status, SessionStatus::Risk);
    assert_eq!(state.status_reason, "此设备不允许读取流量计数");
    assert!(state.speed.is_none());
}

#[test]
fn ignores_unavailable_connected_count_for_session_health() {
    let mut reducer = SessionReducer::new(5);
    let mut sample = baseline();
    sample.connected_device_count = MetricAvailability::Unavailable {
        reason: "Connected count restricted by system".into(),
    };
    reducer.accept(sample);

    let mut second = baseline();
    second.timestamp_millis = 2_000;
    second.traffic = MetricAvailability::Available(TrafficReading {
        rx_bytes: 3_000,
        tx_bytes: 1_500,
    });
    second.connected_device_count = MetricAvailability::Unavailable {
        reason: "Connected count restricted by system".into(),
    };
    let state = reducer.accept(second);

    assert_eq!(state.status, SessionStatus::Stable);
    assert_eq!(state.status_reason, "热点会话状态稳定");
}

#[test]
fn marks_critical_battery_as_risk() {
    let mut reducer = SessionReducer::new(5);
    let mut sample = baseline();
    sample.battery = MetricAvailability::Available(BatteryReading {
        level_percent: Some(8),
        temperature_celsius: Some(36.0),
        is_charging: false,
    });

    let state = reducer.accept(sample);

    assert_eq!(state.status, SessionStatus::Risk);
    assert_eq!(state.status_reason, "电量严重偏低");
}

#[test]
fn keeps_speed_trend_bounded() {
    let mut reducer = SessionReducer::new(2);
    reducer.accept(baseline());

    let mut second = baseline();
    second.timestamp_millis = 2_000;
    second.traffic = MetricAvailability::Available(TrafficReading {
        rx_bytes: 2_000,
        tx_bytes: 600,
    });
    reducer.accept(second);

    let mut third = baseline();
    third.timestamp_millis = 3_000;
    third.traffic = MetricAvailability::Available(TrafficReading {
        rx_bytes: 4_500,
        tx_bytes: 900,
    });
    let state = reducer.accept(third);

    assert_eq!(state.speed_trend.len(), 2);
    assert_eq!(
        state.speed_trend.last().unwrap().down_bytes_per_second,
        2_500
    );
}
