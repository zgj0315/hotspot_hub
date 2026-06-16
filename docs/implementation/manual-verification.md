# Hotspot Hub 手工验证

## 准备

- Android 手机已开启 Wi-Fi 热点，并开启 USB 调试。
- 电脑已安装 Rust、`cargo-apk`、Android SDK Platform 36 和 Android NDK。
- 本机 NDK 路径：

```sh
/Users/zhaogj/Library/Android/sdk/ndk/30.0.14904198
```

首次准备：

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk
adb devices
```

如果连接了多台设备，后续 `adb` 命令加上 `-s <serial>`。

## 编译

Debug 包：

```sh
ANDROID_NDK_ROOT=/Users/zhaogj/Library/Android/sdk/ndk/30.0.14904198 \
ANDROID_NDK=/Users/zhaogj/Library/Android/sdk/ndk/30.0.14904198 \
cargo apk build --lib --target aarch64-linux-android
```

输出：

```sh
target/debug/apk/HotspotHub.apk
```

Release 包：

```sh
ANDROID_NDK_ROOT=/Users/zhaogj/Library/Android/sdk/ndk/30.0.14904198 \
ANDROID_NDK=/Users/zhaogj/Library/Android/sdk/ndk/30.0.14904198 \
CARGO_APK_RELEASE_KEYSTORE=/Users/zhaogj/.android/debug.keystore \
CARGO_APK_RELEASE_KEYSTORE_PASSWORD=android \
cargo apk build --release --lib --target aarch64-linux-android
```

输出：

```sh
target/release/apk/HotspotHub.apk
```

说明：这里的 release 包使用 debug keystore，只适合测试机安装。正式发布必须换成正式 keystore，且不能把 keystore 或密码提交到仓库。

## 安装和启动

安装 release 包：

```sh
adb install -r target/release/apk/HotspotHub.apk
adb shell monkey -p com.example.hotspothub 1
```

如果签名不兼容，测试机可先卸载再安装：

```sh
adb uninstall com.example.hotspothub
adb install target/release/apk/HotspotHub.apk
```

## 冒烟验证

手机界面检查：

- 首页为只读信息面板，没有热点控制按钮。
- 界面文字为简体中文，布局在手机屏幕中正常居中。
- 已运行时间递增，实时速度约每秒刷新。
- 会话流量、电量、温度可见。
- 设备数量来自 Android netlink/ARP 或 `ip neigh` 的 best-effort 估算，显示 `N 台（估算）`、`0 台（估算）` 或 `系统限制`。
- `ip neigh` 路径只统计热点网卡上的 IPv4 活跃邻居，避免把已断开的 IPv6 缓存算进去。
- Home/锁屏后重新打开，应用不崩溃。

ADB 检查：

```sh
adb shell pidof com.example.hotspothub
adb shell dumpsys activity activities
adb logcat -d -e FATAL
adb shell screencap -p /sdcard/hotspot_hub.png
adb pull /sdcard/hotspot_hub.png /tmp/hotspot_hub.png
```

预期：应用进程存在，Activity 为 `RESUMED`，logcat 没有 `FATAL`。

## 最近一次记录

- 日期：2026-06-17
- 设备：`30906b3e`
- APK：`target/release/apk/HotspotHub.apk`
- 结果：release 包安装成功，应用前台运行，未发现 `FATAL`。当前测试机通过 `ip neigh` 路径显示 `2 台（估算）`，与 `dumpsys wifi` 当前关联数一致。
