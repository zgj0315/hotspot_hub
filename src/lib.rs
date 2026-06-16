pub mod connected_devices;
pub mod format;
pub mod model;
pub mod monitor;
pub mod platform;
pub mod reducer;
pub mod ring_buffer;
pub mod sources;

slint::include_modules!();

#[cfg(target_os = "android")]
use crate::monitor::set_foreground_active;
use crate::monitor::{start_monitor, HotspotMonitor};
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
                    AndroidConnectedDeviceCountSource::new(),
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
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("HotspotHub"),
    );
    slint::android::init_with_event_listener(app, |event| match event {
        PollEvent::Main(MainEvent::Resume { .. }) | PollEvent::Main(MainEvent::Start) => {
            set_foreground_active(true);
        }
        PollEvent::Main(MainEvent::Pause) | PollEvent::Main(MainEvent::Stop) => {
            set_foreground_active(false);
        }
        _ => {}
    })
    .expect("failed to initialize Slint Android backend");
    run_app().expect("failed to run Hotspot Hub");
}
