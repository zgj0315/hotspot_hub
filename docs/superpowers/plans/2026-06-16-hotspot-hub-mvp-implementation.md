# Hotspot Hub MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first Hotspot Hub MVP as a Slint + Rust Android app that displays a read-only hotspot-session dashboard with duration, estimated traffic, real-time speed, battery, temperature, connected-count degradation, bounded trends, and adaptive refresh.

**Architecture:** Use Rust as the product and platform core: pure Rust modules own metric models, session reduction, status classification, formatting, and sampling cadence. Slint owns the UI in `.slint` files and is updated from Rust properties. Android packaging uses Slint's Android activity backend from Rust, with desktop execution kept as a fast development and preview path.

**Tech Stack:** Rust 2021, Slint Rust crate with Android activity backend, `slint-build`, `android-activity`/Slint Android integration, `cargo-apk` for Android packaging, Rust unit tests.

---

## File Structure

- `Cargo.toml`: Rust package, Slint dependencies, desktop binary, Android cdylib metadata, and Android package metadata.
- `build.rs`: compiles `ui/hotspot_hub.slint`.
- `.cargo/config.toml`: optional Android linker and target configuration notes.
- `src/lib.rs`: shared app entry, Slint include, state wiring, Android `android_main`.
- `src/main.rs`: desktop/dev entry point.
- `src/model.rs`: pure Rust metric models, dashboard state, availability, status enums.
- `src/ring_buffer.rs`: bounded in-memory buffer for trends.
- `src/reducer.rs`: pure reducer from samples to dashboard state.
- `src/sources.rs`: source traits for clock, traffic, battery, connected count.
- `src/platform/mod.rs`: platform module selector.
- `src/platform/desktop.rs`: desktop fake sources for development preview.
- `src/platform/android.rs`: Android sources and explicit degradation boundaries.
- `src/monitor.rs`: adaptive sampler and Slint property updater.
- `src/format.rs`: display formatting helpers.
- `ui/hotspot_hub.slint`: read-only dashboard UI.
- `tests/ring_buffer_test.rs`: bounded-buffer tests.
- `tests/reducer_test.rs`: speed, traffic, status, and degradation tests.
- `tests/format_test.rs`: formatting tests.
- `docs/implementation/manual-verification.md`: real-device verification checklist.

## External References

- Slint official Rust example uses `slint_build::compile(...)`, `slint::include_modules!()`, and an Android entry point shaped as `android_main(app: slint::android::AndroidApp)`.
- Slint official Rust example enables the `backend-android-activity-06` feature for Android activity integration.
- Android product metrics remain constrained by the approved product spec: `TrafficStats`-style device counters are an estimate, battery state comes from Android battery APIs, and connected-count must degrade explicitly when unsupported.

### Task 1: Scaffold Slint + Rust Project

**Files:**
- Create: `Cargo.toml`
- Create: `build.rs`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `ui/hotspot_hub.slint`

- [ ] **Step 1: Create Cargo package configuration**

Create `Cargo.toml`:

```toml
[package]
name = "hotspot_hub"
version = "0.1.0"
edition = "2021"
build = "build.rs"
publish = false

[lib]
name = "hotspot_hub"
path = "src/lib.rs"
crate-type = ["lib", "cdylib"]

[[bin]]
name = "hotspot-hub"
path = "src/main.rs"

[dependencies]
slint = { version = "1.17", features = ["backend-android-activity-06"] }

[target.'cfg(target_os = "android")'.dependencies]
android-activity = { version = "0.6", features = ["native-activity"] }
android_logger = "0.15"
jni = "0.21"
log = "0.4"
ndk-context = "0.1"

[build-dependencies]
slint-build = "1.17"

[package.metadata.android]
package = "com.example.hotspothub"
apk_name = "HotspotHub"
assets = "assets"
build_targets = ["aarch64-linux-android"]
resources = "android-res"
strip = "strip"
runtime_libs = "target/android-libs"

[package.metadata.android.sdk]
min_sdk_version = 26
target_sdk_version = 35
```

- [ ] **Step 2: Create Slint build script**

Create `build.rs`:

```rust
fn main() {
    slint_build::compile("ui/hotspot_hub.slint").expect("failed to compile Slint UI");
}
```

- [ ] **Step 3: Create minimal Slint UI**

Create `ui/hotspot_hub.slint`:

```slint
import { VerticalBox, Text } from "std-widgets.slint";

export component MainWindow inherits Window {
    title: "Hotspot Hub";
    width: 360px;
    height: 720px;

    in property <string> status: "Unknown";
    in property <string> status-reason: "Waiting for samples";

    VerticalBox {
        padding: 24px;
        spacing: 12px;

        Text {
            text: "Hotspot Hub";
            font-size: 18px;
            color: #94a3b8;
        }

        Text {
            text: root.status;
            font-size: 32px;
            font-weight: 700;
            color: #f8fafc;
        }

        Text {
            text: root.status-reason;
            font-size: 16px;
            color: #cbd5e1;
        }
    }

    background: #0f172a;
}
```

- [ ] **Step 4: Create Rust library entry**

Create `src/lib.rs`:

```rust
slint::include_modules!();

pub fn run_app() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;
    window.set_status("Unknown".into());
    window.set_status_reason("Waiting for samples".into());
    window.run()
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("failed to initialize Slint Android backend");
    run_app().expect("failed to run Hotspot Hub");
}
```

- [ ] **Step 5: Create desktop/dev binary**

Create `src/main.rs`:

```rust
fn main() -> Result<(), slint::PlatformError> {
    hotspot_hub::run_app()
}
```

- [ ] **Step 6: Run desktop build checks**

Run:

```bash
cargo test
cargo build
```

Expected: both commands exit 0.

- [ ] **Step 7: Commit scaffold**

Run:

```bash
git add Cargo.toml build.rs src ui
git commit -m "chore: scaffold Slint Rust app"
```

Expected: commit succeeds.

### Task 2: Add Core Models And Bounded Trend Buffer

**Files:**
- Create: `src/model.rs`
- Create: `src/ring_buffer.rs`
- Modify: `src/lib.rs`
- Test: `tests/ring_buffer_test.rs`

- [ ] **Step 1: Write failing ring buffer tests**

Create `tests/ring_buffer_test.rs`:

```rust
use hotspot_hub::ring_buffer::RingBuffer;

#[test]
fn keeps_only_newest_items() {
    let mut buffer = RingBuffer::new(3);
    buffer.push(1);
    buffer.push(2);
    buffer.push(3);
    buffer.push(4);

    assert_eq!(buffer.to_vec(), vec![2, 3, 4]);
}

#[test]
fn zero_capacity_stays_empty() {
    let mut buffer = RingBuffer::new(0);
    buffer.push(1);

    assert_eq!(buffer.to_vec(), Vec::<i32>::new());
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
cargo test --test ring_buffer_test
```

Expected: FAIL because `hotspot_hub::ring_buffer::RingBuffer` is not defined.

- [ ] **Step 3: Add metric and dashboard models**

Create `src/model.rs`:

```rust
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
            status_reason: "Waiting for enough samples".into(),
            session_duration_millis: 0,
            session_rx_bytes: None,
            session_tx_bytes: None,
            speed: None,
            connected_device_count: MetricAvailability::Unavailable {
                reason: "Waiting for enough samples".into(),
            },
            battery: MetricAvailability::Unavailable {
                reason: "Waiting for enough samples".into(),
            },
            speed_trend: Vec::new(),
            battery_trend: Vec::new(),
            temperature_trend: Vec::new(),
            last_updated_millis: None,
        }
    }
}
```

- [ ] **Step 4: Add ring buffer**

Create `src/ring_buffer.rs`:

```rust
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct RingBuffer<T> {
    capacity: usize,
    values: VecDeque<T>,
}

impl<T: Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            values: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, value: T) {
        if self.capacity == 0 {
            return;
        }
        while self.values.len() >= self.capacity {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.values.iter().cloned().collect()
    }
}
```

- [ ] **Step 5: Export modules**

Modify the top of `src/lib.rs` so it starts with:

```rust
pub mod model;
pub mod ring_buffer;

slint::include_modules!();
```

Keep the rest of `src/lib.rs` unchanged.

- [ ] **Step 6: Run tests and verify they pass**

Run:

```bash
cargo test --test ring_buffer_test
cargo test
```

Expected: both commands PASS.

- [ ] **Step 7: Commit models**

Run:

```bash
git add src/model.rs src/ring_buffer.rs src/lib.rs tests/ring_buffer_test.rs
git commit -m "feat: add monitoring models and bounded trends"
```

Expected: commit succeeds.

### Task 3: Implement Session Reducer And Status Classification

**Files:**
- Create: `src/reducer.rs`
- Modify: `src/lib.rs`
- Test: `tests/reducer_test.rs`

- [ ] **Step 1: Write failing reducer tests**

Create `tests/reducer_test.rs`:

```rust
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
    assert_eq!(state.status_reason, "Traffic counters unavailable on this device");
    assert!(state.speed.is_none());
}

#[test]
fn marks_connected_count_unavailable_as_attention() {
    let mut reducer = SessionReducer::new(5);
    let mut sample = baseline();
    sample.connected_device_count = MetricAvailability::Unavailable {
        reason: "Connected count restricted by system".into(),
    };

    let state = reducer.accept(sample);

    assert_eq!(state.status, SessionStatus::Attention);
    assert_eq!(state.status_reason, "Connected count restricted by system");
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
    assert_eq!(state.status_reason, "Battery critically low");
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
    assert_eq!(state.speed_trend.last().unwrap().down_bytes_per_second, 2_500);
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
cargo test --test reducer_test
```

Expected: FAIL because `SessionReducer` is not defined.

- [ ] **Step 3: Add reducer implementation**

Create `src/reducer.rs`:

```rust
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
        let elapsed = current.timestamp_millis.checked_sub(previous.timestamp_millis)?;
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
            Some(current_traffic.rx_bytes.saturating_sub(base_traffic.rx_bytes)),
            Some(current_traffic.tx_bytes.saturating_sub(base_traffic.tx_bytes)),
        )
    }

    fn classify(
        &self,
        sample: &MetricSample,
        speed: Option<&SpeedSnapshot>,
    ) -> (SessionStatus, String) {
        if let MetricAvailability::Unavailable { reason } = &sample.traffic {
            return (SessionStatus::Risk, reason.clone());
        }

        let battery = match &sample.battery {
            MetricAvailability::Available(value) => value,
            MetricAvailability::Unavailable { reason } => {
                return (SessionStatus::Attention, reason.clone());
            }
        };

        if battery.level_percent.is_some_and(|level| level < 10) && !battery.is_charging {
            return (SessionStatus::Risk, "Battery critically low".into());
        }
        if battery
            .temperature_celsius
            .is_some_and(|temperature| temperature >= 45.0)
        {
            return (SessionStatus::Risk, "Temperature high".into());
        }
        if battery.level_percent.is_some_and(|level| level < 20) {
            return (SessionStatus::Attention, "Battery low".into());
        }
        if battery
            .temperature_celsius
            .is_some_and(|temperature| temperature >= 40.0)
        {
            return (SessionStatus::Attention, "Temperature elevated".into());
        }
        if let MetricAvailability::Unavailable { reason } = &sample.connected_device_count {
            return (SessionStatus::Attention, reason.clone());
        }
        if speed.is_none() {
            return (SessionStatus::Unknown, "Waiting for enough samples".into());
        }

        (SessionStatus::Stable, "Hotspot session looks stable".into())
    }
}
```

- [ ] **Step 4: Export reducer module**

Modify `src/lib.rs` module exports:

```rust
pub mod model;
pub mod reducer;
pub mod ring_buffer;
```

Keep the `slint::include_modules!()` line and app entry code unchanged.

- [ ] **Step 5: Run reducer and all tests**

Run:

```bash
cargo test --test reducer_test
cargo test
```

Expected: both commands PASS.

- [ ] **Step 6: Commit reducer**

Run:

```bash
git add src/reducer.rs src/lib.rs tests/reducer_test.rs
git commit -m "feat: reduce samples into dashboard state"
```

Expected: commit succeeds.

### Task 4: Add Metric Sources And Android JNI Bridge

**Files:**
- Create: `src/sources.rs`
- Create: `src/platform/mod.rs`
- Create: `src/platform/desktop.rs`
- Create: `src/platform/android.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add source traits**

Create `src/sources.rs`:

```rust
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
```

- [ ] **Step 2: Add desktop development sources**

Create `src/platform/desktop.rs`:

```rust
use crate::model::{BatteryReading, MetricAvailability, TrafficReading};
use crate::sources::{BatterySource, ConnectedDeviceCountSource, TrafficSource};

pub struct DesktopTrafficSource {
    rx: u64,
    tx: u64,
}

impl DesktopTrafficSource {
    pub fn new() -> Self {
        Self { rx: 1_000_000, tx: 250_000 }
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
```

- [ ] **Step 3: Add Android JNI metric sources**

Create `src/platform/android.rs`:

```rust
use crate::model::{BatteryReading, MetricAvailability, TrafficReading};
use crate::sources::{BatterySource, ConnectedDeviceCountSource, TrafficSource};
use jni::objects::{JObject, JValue};
use jni::sys::jobject;
use jni::JavaVM;

pub struct AndroidTrafficSource;

impl TrafficSource for AndroidTrafficSource {
    fn read_traffic(&mut self) -> MetricAvailability<TrafficReading> {
        read_android_traffic().unwrap_or_else(|reason| MetricAvailability::Unavailable { reason })
    }
}

pub struct AndroidBatterySource;

impl BatterySource for AndroidBatterySource {
    fn read_battery(&mut self) -> MetricAvailability<BatteryReading> {
        read_android_battery().unwrap_or_else(|reason| MetricAvailability::Unavailable { reason })
    }
}

pub struct AndroidConnectedDeviceCountSource;

impl ConnectedDeviceCountSource for AndroidConnectedDeviceCountSource {
    fn read_connected_device_count(&mut self) -> MetricAvailability<u32> {
        MetricAvailability::Unavailable {
            reason: "Connected count restricted by system".into(),
        }
    }
}

fn java_vm() -> Result<JavaVM, String> {
    let ctx = ndk_context::android_context();
    unsafe { JavaVM::from_raw(ctx.vm().cast()) }
        .map_err(|error| format!("Android JavaVM unavailable: {error}"))
}

fn read_android_traffic() -> Result<MetricAvailability<TrafficReading>, String> {
    let vm = java_vm()?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| format!("JNI attach failed: {error}"))?;

    let rx = env
        .call_static_method("android/net/TrafficStats", "getTotalRxBytes", "()J", &[])
        .and_then(|value| value.j())
        .map_err(|error| format!("Traffic counters unavailable on this device: {error}"))?;
    let tx = env
        .call_static_method("android/net/TrafficStats", "getTotalTxBytes", "()J", &[])
        .and_then(|value| value.j())
        .map_err(|error| format!("Traffic counters unavailable on this device: {error}"))?;

    if rx < 0 || tx < 0 {
        return Ok(MetricAvailability::Unavailable {
            reason: "Traffic counters unavailable on this device".into(),
        });
    }

    Ok(MetricAvailability::Available(TrafficReading {
        rx_bytes: rx as u64,
        tx_bytes: tx as u64,
    }))
}

fn read_android_battery() -> Result<MetricAvailability<BatteryReading>, String> {
    let ctx = ndk_context::android_context();
    let vm = java_vm()?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| format!("JNI attach failed: {error}"))?;

    let action = env
        .get_static_field(
            "android/content/Intent",
            "ACTION_BATTERY_CHANGED",
            "Ljava/lang/String;",
        )
        .and_then(|value| value.l())
        .map_err(|error| format!("Battery intent action unavailable: {error}"))?;
    let filter = env
        .new_object(
            "android/content/IntentFilter",
            "(Ljava/lang/String;)V",
            &[JValue::Object(&action)],
        )
        .map_err(|error| format!("Battery intent filter unavailable: {error}"))?;
    let context = unsafe { JObject::from_raw(ctx.context() as jobject) };
    let intent = env
        .call_method(
            &context,
            "registerReceiver",
            "(Landroid/content/BroadcastReceiver;Landroid/content/IntentFilter;)Landroid/content/Intent;",
            &[JValue::Object(&JObject::null()), JValue::Object(&filter)],
        )
        .and_then(|value| value.l())
        .map_err(|error| format!("Battery state unavailable: {error}"))?;

    if intent.is_null() {
        return Ok(MetricAvailability::Unavailable {
            reason: "Battery state unavailable".into(),
        });
    }

    let level = int_extra(&mut env, &intent, "EXTRA_LEVEL", -1)?;
    let scale = int_extra(&mut env, &intent, "EXTRA_SCALE", -1)?;
    let temperature = int_extra(&mut env, &intent, "EXTRA_TEMPERATURE", i32::MIN)?;
    let status = int_extra(&mut env, &intent, "EXTRA_STATUS", 1)?;

    let level_percent = if level >= 0 && scale > 0 {
        Some(((level * 100 / scale).clamp(0, 100)) as u8)
    } else {
        None
    };
    let temperature_celsius = if temperature == i32::MIN {
        None
    } else {
        Some(temperature as f32 / 10.0)
    };
    let is_charging = status == 2 || status == 5;

    Ok(MetricAvailability::Available(BatteryReading {
        level_percent,
        temperature_celsius,
        is_charging,
    }))
}

fn int_extra(
    env: &mut jni::JNIEnv<'_>,
    intent: &JObject<'_>,
    field_name: &str,
    default_value: i32,
) -> Result<i32, String> {
    let key = env
        .get_static_field("android/os/BatteryManager", field_name, "Ljava/lang/String;")
        .and_then(|value| value.l())
        .map_err(|error| format!("Battery field {field_name} unavailable: {error}"))?;
    env.call_method(
        intent,
        "getIntExtra",
        "(Ljava/lang/String;I)I",
        &[JValue::Object(&key), JValue::Int(default_value)],
    )
    .and_then(|value| value.i())
    .map_err(|error| format!("Battery field {field_name} read failed: {error}"))
}
```

- [ ] **Step 4: Add platform module selector**

Create `src/platform/mod.rs`:

```rust
#[cfg(target_os = "android")]
pub mod android;

#[cfg(not(target_os = "android"))]
pub mod desktop;
```

- [ ] **Step 5: Export source and platform modules**

Modify `src/lib.rs` module exports:

```rust
pub mod model;
pub mod platform;
pub mod reducer;
pub mod ring_buffer;
pub mod sources;
```

Keep the Slint include and app entry code unchanged.

- [ ] **Step 6: Run tests**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 7: Commit source boundaries**

Run:

```bash
git add src/sources.rs src/platform src/lib.rs
git commit -m "feat: add metric source boundaries"
```

Expected: commit succeeds.

### Task 5: Add Formatting Helpers

**Files:**
- Create: `src/format.rs`
- Modify: `src/lib.rs`
- Test: `tests/format_test.rs`

- [ ] **Step 1: Write failing formatter tests**

Create `tests/format_test.rs`:

```rust
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
```

- [ ] **Step 2: Run tests and verify they fail**

Run:

```bash
cargo test --test format_test
```

Expected: FAIL because `hotspot_hub::format` is not defined.

- [ ] **Step 3: Add formatter implementation**

Create `src/format.rs`:

```rust
pub fn format_duration(milliseconds: u64) -> String {
    let total_minutes = milliseconds / 60_000;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
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
```

- [ ] **Step 4: Export formatter module**

Modify `src/lib.rs` module exports:

```rust
pub mod format;
pub mod model;
pub mod platform;
pub mod reducer;
pub mod ring_buffer;
pub mod sources;
```

Keep the rest unchanged.

- [ ] **Step 5: Run formatter and all tests**

Run:

```bash
cargo test --test format_test
cargo test
```

Expected: both commands PASS.

- [ ] **Step 6: Commit formatters**

Run:

```bash
git add src/format.rs src/lib.rs tests/format_test.rs
git commit -m "feat: add dashboard formatters"
```

Expected: commit succeeds.

### Task 6: Build Full Slint Dashboard UI

**Files:**
- Replace: `ui/hotspot_hub.slint`
- Modify: `src/lib.rs`

- [ ] **Step 1: Replace Slint UI with read-only dashboard**

Replace `ui/hotspot_hub.slint`:

```slint
import { VerticalBox, HorizontalBox, Text, Rectangle } from "std-widgets.slint";

component MetricCard inherits Rectangle {
    in property <string> label;
    in property <string> value;

    border-radius: 14px;
    background: #1e293b;
    height: 92px;

    VerticalBox {
        padding: 12px;
        spacing: 6px;

        Text {
            text: root.label;
            color: #94a3b8;
            font-size: 13px;
        }

        Text {
            text: root.value;
            color: #f8fafc;
            font-size: 24px;
            font-weight: 700;
        }
    }
}

component SectionCard inherits Rectangle {
    border-radius: 18px;
    background: #1e293b;
}

export component MainWindow inherits Window {
    title: "Hotspot Hub";
    width: 360px;
    height: 720px;
    background: #0f172a;

    in property <string> status: "Unknown";
    in property <string> status-reason: "Waiting for samples";
    in property <string> elapsed: "0m";
    in property <string> session-traffic: "--";
    in property <string> down-speed: "--";
    in property <string> up-speed: "--";
    in property <string> connected-count: "Restricted";
    in property <string> battery: "--";
    in property <string> temperature: "--";
    in property <string> speed-trend: "Waiting for samples";
    in property <string> battery-trend: "Battery samples: 0";
    in property <string> temperature-trend: "Temperature samples: 0";
    in property <string> last-updated: "Waiting for first sample";

    VerticalBox {
        padding: 16px;
        spacing: 12px;

        HorizontalBox {
            spacing: 10px;

            VerticalBox {
                Text {
                    text: "Hotspot Hub";
                    color: #94a3b8;
                    font-size: 14px;
                }
                Text {
                    text: "Hotspot session";
                    color: #f8fafc;
                    font-size: 26px;
                    font-weight: 700;
                }
            }

            Rectangle {
                width: 96px;
                height: 34px;
                border-radius: 17px;
                background: status == "Stable" ? #14532d : status == "Attention" ? #854d0e : status == "Risk" ? #7f1d1d : #334155;
                Text {
                    text: status;
                    color: white;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                    font-size: 13px;
                }
            }
        }

        Text {
            text: status-reason;
            color: #cbd5e1;
            font-size: 14px;
            wrap: word-wrap;
        }

        HorizontalBox {
            spacing: 10px;
            MetricCard { label: "Elapsed"; value: elapsed; }
            MetricCard { label: "Session traffic"; value: session-traffic; }
        }

        SectionCard {
            height: 160px;
            VerticalBox {
                padding: 14px;
                spacing: 10px;
                Text { text: "Real-time speed"; color: #cbd5e1; font-size: 16px; font-weight: 700; }
                HorizontalBox {
                    spacing: 10px;
                    MetricCard { label: "Down"; value: down-speed; }
                    MetricCard { label: "Up"; value: up-speed; }
                }
                Text { text: speed-trend; color: #38bdf8; font-size: 14px; }
            }
        }

        HorizontalBox {
            spacing: 8px;
            MetricCard { label: "Devices"; value: connected-count; }
            MetricCard { label: "Battery"; value: battery; }
            MetricCard { label: "Temp"; value: temperature; }
        }

        SectionCard {
            height: 98px;
            VerticalBox {
                padding: 14px;
                spacing: 6px;
                Text { text: "Battery and temperature"; color: #cbd5e1; font-size: 16px; font-weight: 700; }
                Text { text: battery-trend; color: #94a3b8; font-size: 13px; }
                Text { text: temperature-trend; color: #94a3b8; font-size: 13px; }
            }
        }

        Text {
            text: last-updated;
            color: #94a3b8;
            font-size: 12px;
        }
    }
}
```

- [ ] **Step 2: Build after UI replacement**

Run:

```bash
cargo build
cargo test
```

Expected: both commands PASS.

- [ ] **Step 3: Commit dashboard UI**

Run:

```bash
git add ui/hotspot_hub.slint
git commit -m "feat: add Slint dashboard UI"
```

Expected: commit succeeds.

### Task 7: Add Adaptive Monitor And Wire UI Properties

**Files:**
- Create: `src/monitor.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add adaptive monitor**

Create `src/monitor.rs`:

```rust
use crate::format::{format_bytes, format_duration, format_speed};
use crate::model::{BatteryReading, DashboardState, MetricAvailability, MetricSample, SessionStatus};
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
)
where
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
    window.set_status(status_text(state.status).into());
    window.set_status_reason(state.status_reason.clone().into());
    window.set_elapsed(format_duration(state.session_duration_millis).into());
    window.set_session_traffic(
        format_bytes(total_bytes(state.session_rx_bytes, state.session_tx_bytes)).into(),
    );
    window.set_down_speed(format_speed(state.speed.as_ref().map(|speed| speed.down_bytes_per_second)).into());
    window.set_up_speed(format_speed(state.speed.as_ref().map(|speed| speed.up_bytes_per_second)).into());
    window.set_connected_count(format_connected(&state.connected_device_count).into());
    window.set_battery(format_battery(&state.battery).into());
    window.set_temperature(format_temperature(&state.battery).into());
    window.set_speed_trend(format!("Speed samples: {}", state.speed_trend.len()).into());
    window.set_battery_trend(format!("Battery samples: {}", state.battery_trend.len()).into());
    window.set_temperature_trend(format!("Temperature samples: {}", state.temperature_trend.len()).into());
    window.set_last_updated(match state.last_updated_millis {
        Some(value) => SharedString::from(format!("Updated {value}")),
        None => SharedString::from("Waiting for first sample"),
    });
}

fn status_text(status: SessionStatus) -> &'static str {
    match status {
        SessionStatus::Stable => "Stable",
        SessionStatus::Attention => "Attention",
        SessionStatus::Risk => "Risk",
        SessionStatus::Unknown => "Unknown",
    }
}

fn total_bytes(rx: Option<u64>, tx: Option<u64>) -> Option<u64> {
    Some(rx? + tx?)
}

fn format_connected(value: &MetricAvailability<u32>) -> String {
    match value {
        MetricAvailability::Available(count) => count.to_string(),
        MetricAvailability::Unavailable { .. } => "Restricted".into(),
    }
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
            .map(|temperature| format!("{temperature:.0}C"))
            .unwrap_or_else(|| "--".into()),
        MetricAvailability::Unavailable { .. } => "--".into(),
    }
}
```

- [ ] **Step 2: Wire monitor into app entry**

Replace `src/lib.rs`:

```rust
pub mod format;
pub mod model;
pub mod monitor;
pub mod platform;
pub mod reducer;
pub mod ring_buffer;
pub mod sources;

slint::include_modules!();

use crate::monitor::{set_foreground_active, start_monitor, HotspotMonitor};
use crate::sources::SystemClock;

pub fn run_app() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;

    let _monitor_timer = {
        #[cfg(target_os = "android")]
        {
            use crate::platform::android::{
                AndroidBatterySource, AndroidConnectedDeviceCountSource, AndroidTrafficSource,
            };
            start_monitor(
                &window,
                HotspotMonitor::new(
                    AndroidTrafficSource,
                    AndroidBatterySource,
                    AndroidConnectedDeviceCountSource,
                    SystemClock,
                ),
            )
        }

        #[cfg(not(target_os = "android"))]
        {
            use crate::platform::desktop::{
                DesktopBatterySource, DesktopConnectedDeviceCountSource, DesktopTrafficSource,
            };
            start_monitor(
                &window,
                HotspotMonitor::new(
                    DesktopTrafficSource::new(),
                    DesktopBatterySource,
                    DesktopConnectedDeviceCountSource,
                    SystemClock,
                ),
            )
        }
    };

    window.run()
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: slint::android::AndroidApp) {
    use slint::android::android_activity::{MainEvent, PollEvent};

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    slint::android::init_with_event_listener(app, |event| {
        match event {
            PollEvent::Main(MainEvent::Resume { .. }) | PollEvent::Main(MainEvent::Start) => {
                set_foreground_active(true);
            }
            PollEvent::Main(MainEvent::Pause) | PollEvent::Main(MainEvent::Stop) => {
                set_foreground_active(false);
            }
            _ => {}
        }
    })
    .expect("failed to initialize Slint Android backend");
    run_app().expect("failed to run Hotspot Hub");
}
```

- [ ] **Step 3: Run tests and desktop build**

Run:

```bash
cargo test
cargo build
```

Expected: both commands PASS.

- [ ] **Step 4: Commit monitor wiring**

Run:

```bash
git add src/monitor.rs src/lib.rs
git commit -m "feat: wire adaptive monitor to Slint UI"
```

Expected: commit succeeds.

### Task 8: Verify Android Packaging Path

**Files:**
- Create: `.cargo/config.toml`
- Modify: `Cargo.toml` if Android packaging validation requires metadata corrections.

- [ ] **Step 1: Add Android target notes**

Create `.cargo/config.toml`:

```toml
[target.aarch64-linux-android]
linker = "aarch64-linux-android-clang"
```

- [ ] **Step 2: Install Android target if missing**

Run:

```bash
rustup target add aarch64-linux-android
```

Expected: target is installed or already up to date.

- [ ] **Step 3: Install cargo-apk if missing**

Run:

```bash
cargo install cargo-apk
```

Expected: `cargo-apk` installs successfully or is already installed.

- [ ] **Step 4: Build Android APK**

Run:

```bash
cargo apk build --target aarch64-linux-android
```

Expected: command exits 0 and produces a debug APK under `target/`.

- [ ] **Step 5: Fix package metadata only if cargo-apk reports a concrete metadata error**

If Step 4 reports a metadata error, modify only `[package.metadata.android]` in `Cargo.toml` according to the exact error message. Then rerun:

```bash
cargo apk build --target aarch64-linux-android
```

Expected: command exits 0.

- [ ] **Step 6: Commit Android packaging**

Run:

```bash
git add Cargo.toml .cargo/config.toml
git commit -m "chore: add Android packaging configuration"
```

Expected: commit succeeds.

### Task 9: Add Manual Verification Checklist

**Files:**
- Create: `docs/implementation/manual-verification.md`

- [ ] **Step 1: Add manual verification doc**

Create `docs/implementation/manual-verification.md`:

```markdown
# Hotspot Hub Manual Verification

## Device Setup

- Use a physical Android device.
- Enable Android system Wi-Fi hotspot from Settings.
- Connect at least one client device to the hotspot.
- Build the APK with `cargo apk build --target aarch64-linux-android`.
- Install the APK with `adb install -r <apk-path>`.

## Foreground Monitoring

- Open Hotspot Hub.
- Confirm the home screen is read-only and has no hotspot controls.
- Confirm elapsed time increases.
- Confirm upload/download speed refreshes about once per second.
- Confirm session traffic increases when a connected client loads data.
- Confirm battery and temperature are visible.
- Confirm connected-device count shows `Restricted` or a count, but never silently displays `0` when unavailable.

## Background And Lock Behavior

- Press Home and leave the app for at least 2 minutes.
- Reopen the app and confirm it did not crash.
- Lock the screen for at least 2 minutes.
- Unlock and reopen the app.
- Confirm the app did not keep a visible high-frequency UI update while backgrounded.

## Long Session

- Run a mixed foreground/background session for 2 hours.
- Confirm the app remains responsive.
- Confirm trend sample counts remain bounded.
- Confirm no repeated disk writes are visible in logcat from Hotspot Hub.
```

- [ ] **Step 2: Run final local checks**

Run:

```bash
cargo test
cargo build
```

Expected: both commands PASS.

- [ ] **Step 3: Commit verification doc**

Run:

```bash
git add docs/implementation/manual-verification.md
git commit -m "docs: add manual verification checklist"
```

Expected: commit succeeds.

### Task 10: Final MVP Verification

**Files:**
- Modify only if verification reveals implementation defects.

- [ ] **Step 1: Run all local verification commands**

Run:

```bash
cargo test
cargo build
cargo apk build --target aarch64-linux-android
```

Expected: all commands PASS.

- [ ] **Step 2: Install on a physical Android device**

Run:

```bash
adb devices
adb install -r <apk-path>
```

Expected: `adb devices` lists a device, and `adb install` exits 0.

- [ ] **Step 3: Launch and inspect app**

Run:

```bash
adb shell monkey -p com.example.hotspothub 1
```

Expected: app launches and shows the Hotspot Hub dashboard.

- [ ] **Step 4: Perform manual verification**

Follow `docs/implementation/manual-verification.md`.

Expected: all checklist items pass, or any failing item is fixed and re-tested before declaring the MVP complete.

- [ ] **Step 5: Commit verification fixes if needed**

If Step 4 required fixes, run:

```bash
git add Cargo.toml src ui docs .cargo
git commit -m "fix: address MVP verification issues"
```

Expected: commit succeeds when fixes exist. If no fixes exist, do not create an empty commit.
