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
