# Hotspot Hub MVP Product Design

Date: 2026-06-16
Status: Draft for user review

## Context

Hotspot Hub is an Android app for a phone that is already being used as a Wi-Fi hotspot for a computer and other devices. The app does not operate the hotspot. It only displays key information while the phone provides internet access for a long-running session.

The MVP is optimized for long-running network sharing. The user wants to know whether the hotspot session is still healthy, how much traffic it has used, whether the phone can keep running, and whether connected-device count is visible.

## Product Goals

- Show a concise, read-only overview of the current hotspot session.
- Make long-running hotspot use easier to monitor without adding operational controls.
- Feel real-time while the app is in the foreground.
- Reduce power impact when the app is backgrounded, locked, or not actively viewed.
- Degrade honestly when Android version, device vendor, or permissions prevent a metric from being read.

## Non-Goals

- Do not create, start, stop, configure, or share hotspot credentials.
- Do not disconnect clients, throttle devices, block devices, or manage routing.
- Do not show per-device names, IP addresses, MAC addresses, or traffic details in the MVP.
- Do not require root, adb, system app privileges, or vendor-specific private APIs.
- Do not pursue second-by-second accuracy while the screen is off or the app is backgrounded.

## Primary Scenario

The user turns on Android system hotspot, leaves the phone connected to power or battery, and uses the hotspot for a computer and other devices over a long period. The user occasionally opens Hotspot Hub to check:

- How long the current hotspot session has been running.
- How much traffic has been used during the session.
- Current upload and download speed.
- Whether the phone is under battery or thermal pressure.
- How many devices are connected, when that count is available.

## MVP Experience

The MVP uses a single read-only home screen with a session-trend layout.

The top area shows session status and elapsed time. The middle area shows cumulative traffic for the current session, real-time upload/download speed, and a compact speed trend. The lower area shows connected device count, battery level, battery temperature, and lightweight battery/temperature trends.

Status is expressed as text such as `Stable`, `Attention`, or `Risk`. The status explains the current reason, but does not include actions or controls.

## Home Screen Information Hierarchy

1. Session status: one of `Stable`, `Attention`, `Risk`, or `Unknown`.
2. Session duration: elapsed time since Hotspot Hub detected the current hotspot-monitoring session.
3. Session traffic: estimated bytes sent and received since the session baseline.
4. Real-time speed: current downlink and uplink speed.
5. Speed trend: lightweight recent trend, enough to show activity direction without detailed analytics.
6. Connected device count: numeric count when available; otherwise a clear unavailable state.
7. Battery and temperature: current battery percentage, charging status when useful, battery temperature, and compact trend.
8. Last updated: timestamp or relative update indicator.

## Data Sources And Metric Semantics

### Traffic And Speed

The MVP estimates real-time speed by sampling Android system network counters and calculating the byte delta between samples. `TrafficStats.getTotalRxBytes()` and `TrafficStats.getTotalTxBytes()` return received and transmitted bytes since device boot, so Hotspot Hub records a baseline at session start and computes session traffic by subtracting that baseline.

This is a pragmatic MVP metric. It represents device-level network movement, not guaranteed hotspot-only accounting. If a future version needs more precise historical usage, it can use `NetworkStatsManager`, but device-level summary queries require usage-stat access and should run off the main thread.

Reference:
- Android `TrafficStats`: https://developer.android.com/reference/android/net/TrafficStats
- Android `NetworkStatsManager`: https://developer.android.com/reference/android/app/usage/NetworkStatsManager

### Battery And Temperature

Battery percentage, charge state, and temperature come from Android battery state APIs. Battery temperature should be displayed in Celsius after converting the platform value as needed.

Reference:
- Android `BatteryManager`: https://developer.android.com/reference/android/os/BatteryManager

### Connected Device Count

The MVP only commits to showing connected device count when the value can be obtained through supported platform signals on the target device. If the count is unavailable because of Android version, vendor behavior, or permission restrictions, the UI must show a degraded state such as `Restricted` or `Unavailable`, not `0`.

The MVP does not show device names, IP addresses, MAC addresses, per-device traffic, or connection history.

## Refresh And Power Strategy

Refresh is adaptive by app state.

Foreground with screen on:
- Speed sampling: about once per second.
- Battery, temperature, and connected count: every 5 to 10 seconds.
- Trend updates: derived from existing samples, with a bounded in-memory ring buffer.

Background, locked, or screen off:
- Stop UI refresh.
- Reduce sampling to about 30 to 60 seconds, or pause nonessential sampling entirely when the system is likely to constrain background work.
- Persist only minimal session state.

Power constraints:
- Do not actively scan Wi-Fi.
- Do not hold wake locks for routine monitoring.
- Do not write every sample to disk.
- Do not run high-frequency background loops.
- Do not request invasive permissions for MVP-only metrics.

## State Model

`Stable` means the app can read core metrics, there is recent traffic or no obvious issue, battery is acceptable, and temperature is within a normal range.

`Attention` means monitoring is still useful, but the user should be aware of a condition. Examples:
- Battery is getting low.
- Battery temperature is elevated.
- Speed has been near zero for a sustained short window.
- Connected-device count is unavailable or restricted.
- The app cannot confidently infer hotspot activity.

`Risk` means continued hotspot use may fail or the monitor cannot provide core value. Examples:
- Battery is critically low and the phone is not charging.
- Temperature is high for a sustained period.
- Core traffic counters are unavailable.
- Hotspot appears not to be active or no network movement is detected for an extended period.

`Unknown` is used during initial loading or when there is insufficient data to classify the session.

Thresholds should start conservative and adjustable in implementation:
- Low battery: below 20%.
- Critical battery: below 10% when not charging.
- Elevated temperature: 40-44 C.
- High temperature: 45 C or above.
- No meaningful traffic: sustained near-zero delta over at least 30 seconds while foreground monitoring is active.

## Error And Degraded States

The app must prefer clear degradation over false precision.

Examples:
- `Connected count restricted by system`.
- `Traffic counters unavailable on this device`.
- `Waiting for enough samples`.
- `Temperature unavailable`.
- `Hotspot activity cannot be confirmed`.

The MVP should not use pop-ups, vibration, or notification alerts for these states. A compact status row and inline metric labels are sufficient.

## Privacy

The MVP stores only local session-monitoring data needed to render the current session and recent trends. It does not collect device identities, client MAC addresses, client names, browsing data, DNS data, or remote analytics.

## Acceptance Criteria

- When Android hotspot is already enabled, the app can show a read-only monitoring home screen.
- The screen displays session duration, estimated session traffic, real-time upload/download speed, battery level, and battery temperature.
- Foreground speed refresh is approximately once per second.
- Battery, temperature, and connected-count refresh is lower frequency than speed.
- Background or locked operation reduces sampling and stops UI refresh.
- Connected-device count degrades explicitly when unavailable.
- Trend buffers are bounded and do not grow indefinitely.
- A 2-hour foreground/background mixed session does not crash and does not perform high-frequency disk writes.
- The UI contains no hotspot controls, no client management controls, and no operational actions.

## Open Implementation Notes

- The exact hotspot-detection mechanism should be validated on the first target Android versions and devices.
- Session start may initially mean "user opened the app while hotspot is believed active" rather than a perfect platform hotspot-start event.
- The first implementation should instrument metric availability so product decisions can be revised from real device behavior.
