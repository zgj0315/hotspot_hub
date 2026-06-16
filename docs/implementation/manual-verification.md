# Hotspot Hub Manual Verification

## Device Setup

- Use a physical Android device.
- Enable Android system Wi-Fi hotspot from Settings.
- Connect at least one client device to the hotspot.
- Build the APK with `cargo apk build --lib --target aarch64-linux-android`.
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
