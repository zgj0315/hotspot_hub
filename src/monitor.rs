use crate::format::{
    format_bytes, format_connected_count, format_duration, format_last_updated, format_speed,
    format_status,
};
use crate::model::{BatteryReading, DashboardState, MetricAvailability, MetricSample};
use crate::reducer::SessionReducer;
use crate::sources::{BatterySource, Clock, ConnectedDeviceCountSource, TrafficSource};
use crate::MainWindow;
use slint::{ComponentHandle, SharedString, Timer, TimerMode, Weak};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const FOREGROUND_INTERVAL: Duration = Duration::from_secs(1);
const BACKGROUND_INTERVAL: Duration = Duration::from_secs(60);
const TREND_CAPACITY: usize = 600;
static FOREGROUND_ACTIVE: AtomicBool = AtomicBool::new(true);

pub struct HotspotMonitor<Traffic, Battery, Connected, Time>
where
    Traffic: TrafficSource + 'static,
    Battery: BatterySource + 'static,
    Connected: ConnectedDeviceCountSource + 'static,
    Time: Clock + 'static,
{
    reducer: SessionReducer,
    traffic: Traffic,
    battery: Battery,
    connected: Connected,
    clock: Time,
}

impl<Traffic, Battery, Connected, Time> HotspotMonitor<Traffic, Battery, Connected, Time>
where
    Traffic: TrafficSource + 'static,
    Battery: BatterySource + 'static,
    Connected: ConnectedDeviceCountSource + 'static,
    Time: Clock + 'static,
{
    pub fn new(traffic: Traffic, battery: Battery, connected: Connected, clock: Time) -> Self {
        Self {
            reducer: SessionReducer::new(TREND_CAPACITY),
            traffic,
            battery,
            connected,
            clock,
        }
    }

    fn sample(&mut self) -> DashboardState {
        self.reducer.accept(MetricSample {
            timestamp_millis: self.clock.now_millis(),
            traffic: self.traffic.read_traffic(),
            battery: self.battery.read_battery(),
            connected_device_count: self.connected.read_connected_device_count(),
        })
    }
}

pub fn set_foreground_active(active: bool) {
    FOREGROUND_ACTIVE.store(active, Ordering::Relaxed);
}

fn is_foreground_active() -> bool {
    FOREGROUND_ACTIVE.load(Ordering::Relaxed)
}

pub fn start_monitor<Traffic, Battery, Connected, Time>(
    window: &MainWindow,
    monitor: HotspotMonitor<Traffic, Battery, Connected, Time>,
) -> Rc<Timer>
where
    Traffic: TrafficSource + 'static,
    Battery: BatterySource + 'static,
    Connected: ConnectedDeviceCountSource + 'static,
    Time: Clock + 'static,
{
    let monitor = Rc::new(RefCell::new(monitor));
    let weak = window.as_weak();
    let timer = Rc::new(Timer::default());
    schedule_next_tick(timer.clone(), weak, monitor);
    timer
}

fn schedule_next_tick<Traffic, Battery, Connected, Time>(
    timer: Rc<Timer>,
    weak: Weak<MainWindow>,
    monitor: Rc<RefCell<HotspotMonitor<Traffic, Battery, Connected, Time>>>,
) where
    Traffic: TrafficSource + 'static,
    Battery: BatterySource + 'static,
    Connected: ConnectedDeviceCountSource + 'static,
    Time: Clock + 'static,
{
    let interval = if is_foreground_active() {
        FOREGROUND_INTERVAL
    } else {
        BACKGROUND_INTERVAL
    };

    timer.start(TimerMode::SingleShot, interval, {
        let timer = timer.clone();
        let weak = weak.clone();
        let monitor = monitor.clone();
        move || {
            let Some(window) = weak.upgrade() else {
                return;
            };
            let state = monitor.borrow_mut().sample();
            if is_foreground_active() {
                apply_state(&window, &state);
            }
            schedule_next_tick(timer.clone(), weak.clone(), monitor.clone());
        }
    });
}

fn apply_state(window: &MainWindow, state: &DashboardState) {
    window.set_status(format_status(state.status).into());
    window.set_status_reason(state.status_reason.clone().into());
    window.set_elapsed(format_duration(state.session_duration_millis).into());
    window.set_session_traffic(
        format_bytes(total_bytes(state.session_rx_bytes, state.session_tx_bytes)).into(),
    );
    window.set_down_speed(
        format_speed(
            state
                .speed
                .as_ref()
                .map(|speed| speed.down_bytes_per_second),
        )
        .into(),
    );
    window.set_up_speed(
        format_speed(state.speed.as_ref().map(|speed| speed.up_bytes_per_second)).into(),
    );
    window.set_connected_count(format_connected_count(&state.connected_device_count).into());
    window.set_battery(format_battery(&state.battery).into());
    window.set_temperature(format_temperature(&state.battery).into());
    window.set_last_updated(SharedString::from(format_last_updated(
        state.last_updated_millis,
    )));
}

fn total_bytes(rx: Option<u64>, tx: Option<u64>) -> Option<u64> {
    Some(rx? + tx?)
}

fn format_battery(value: &MetricAvailability<BatteryReading>) -> String {
    match value {
        MetricAvailability::Available(reading) => reading
            .level_percent
            .map(|level| format!("{level}%"))
            .unwrap_or_else(|| "--".into()),
        MetricAvailability::Unavailable { .. } => "--".into(),
    }
}

fn format_temperature(value: &MetricAvailability<BatteryReading>) -> String {
    match value {
        MetricAvailability::Available(reading) => reading
            .temperature_celsius
            .map(|temperature| format!("{temperature:.0}°C"))
            .unwrap_or_else(|| "--".into()),
        MetricAvailability::Unavailable { .. } => "--".into(),
    }
}
