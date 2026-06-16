pub mod model;
pub mod ring_buffer;

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
