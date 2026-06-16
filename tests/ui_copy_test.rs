#[test]
fn ui_does_not_show_internal_sample_counts() {
    let ui = std::fs::read_to_string(format!(
        "{}/ui/hotspot_hub.slint",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("UI file should be readable");

    assert!(
        !ui.contains("样本"),
        "sample counts are internal telemetry and should not be shown in the MVP UI"
    );
}
