use crate::connected_devices::{
    estimate_count_from_arp, estimate_count_from_dumpsys_wifi, estimate_count_from_ip_neigh,
    estimate_count_from_netlink,
};
use crate::model::{BatteryReading, MetricAvailability, TrafficReading};
use crate::sources::{BatterySource, ConnectedDeviceCountSource, TrafficSource};
use jni::objects::{JObject, JValue};
use jni::sys::jobject;
use jni::JavaVM;
use std::process::Command;
use std::time::{Duration, Instant};

const CONNECTED_COUNT_REFRESH: Duration = Duration::from_secs(10);

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

pub struct AndroidConnectedDeviceCountSource {
    cached: Option<(Instant, MetricAvailability<u32>)>,
}

impl AndroidConnectedDeviceCountSource {
    pub fn new() -> Self {
        Self { cached: None }
    }
}

impl ConnectedDeviceCountSource for AndroidConnectedDeviceCountSource {
    fn read_connected_device_count(&mut self) -> MetricAvailability<u32> {
        if let Some((last_read, cached)) = &self.cached {
            if last_read.elapsed() < CONNECTED_COUNT_REFRESH {
                return cached.clone();
            }
        }

        let value = match read_connected_count_best_effort() {
            Some(count) => MetricAvailability::Available(count),
            None => MetricAvailability::Unavailable {
                reason: "Connected count restricted by system".into(),
            },
        };
        self.cached = Some((Instant::now(), value.clone()));
        value
    }
}

fn read_connected_count_best_effort() -> Option<u32> {
    match estimate_count_from_netlink() {
        Some(count) => {
            log::info!("connected count resolved by netlink: {count}");
            return Some(count);
        }
        None => log::info!("connected count netlink path unavailable"),
    }

    if let Some(count) = read_connected_count_from_ip_command() {
        return Some(count);
    }

    match std::fs::read_to_string("/proc/net/arp") {
        Ok(arp) => {
            let estimate = estimate_count_from_arp(&arp);
            log::info!(
                "connected count ARP path result={estimate:?}; {}",
                summarize_arp_table(&arp)
            );
            if estimate.is_some() {
                return estimate;
            }
        }
        Err(error) => {
            log::warn!("connected count ARP read failed: {error}");
        }
    }

    read_connected_count_from_dumpsys_wifi()
}

fn summarize_arp_table(arp: &str) -> String {
    let entries = arp
        .lines()
        .skip(1)
        .take(8)
        .filter_map(|line| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            if columns.len() < 6 {
                return None;
            }
            Some(format!("flags={} iface={}", columns[2], columns[5]))
        })
        .collect::<Vec<_>>();

    if entries.is_empty() {
        "arp entries=<empty>".into()
    } else {
        format!("arp entries={}", entries.join(", "))
    }
}

fn read_connected_count_from_ip_command() -> Option<u32> {
    let output = run_system_command("/system/bin/ip", &["neigh", "show"])?;
    let estimate = estimate_count_from_ip_neigh(&output);
    log::info!("connected count ip-neigh command result={estimate:?}");
    estimate
}

fn read_connected_count_from_dumpsys_wifi() -> Option<u32> {
    let output = run_system_command("/system/bin/dumpsys", &["wifi"])?;
    let estimate = estimate_count_from_dumpsys_wifi(&output);
    log::info!("connected count dumpsys-wifi command result={estimate:?}");
    estimate
}

fn run_system_command(command: &str, args: &[&str]) -> Option<String> {
    let output = match Command::new(command).args(args).output() {
        Ok(output) => output,
        Err(error) => {
            log::warn!("connected count command failed to start: {command} {args:?}: {error}");
            return None;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = if stderr.trim().is_empty() {
        stdout.into_owned()
    } else {
        format!("{stdout}\n{stderr}")
    };

    log::info!(
        "connected count command finished: {command} {args:?}, status={:?}, stdout_bytes={}, stderr_bytes={}",
        output.status.code(),
        output.stdout.len(),
        output.stderr.len()
    );
    Some(combined)
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
        .get_static_field(
            "android/os/BatteryManager",
            field_name,
            "Ljava/lang/String;",
        )
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
