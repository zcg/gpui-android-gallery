# GPUI Android Gallery

一个跑在 Android 上的 GPUI 组件展示应用。它把 [`gpui-fusion`](https://github.com/cradiy/gpui-fusion)
里可用的控件和视觉效果集中到七个页面里，用来在真机上手感体验、对照渲染效果、观察帧率。

这个仓库不包含 GPUI 本身，只包含应用层代码（约 1600 行 `src/main.rs`），
通过 [`gpuiforge`](https://github.com/cradiy/gpuiforge) 生成 Android 壳工程并构建安装。

---

## 中文

### 这个项目做什么

GPUI 在 Android 上的组件大多只能从文档里了解，改一行看一次效果的成本很高。
这个应用把能用到的控件放到同一份可滑动的界面里，一次安装就能逐个试：

| 页面 | 内容 |
| --- | --- |
| Basics | 字号层级、色板、圆角与阴影、按钮、Badge |
| Inputs | 文本框、密码框、多行输入、数字输入（含步进/范围） |
| Controls | Checkbox、Switch、Radio、分段选择、Slider |
| Glass | Liquid glass 分段控件（水平/垂直）、FrostedGlass 背景模糊、透明度与动效开关 |
| Overlays | Modal 弹窗、Bottom sheet、悬浮卡片 |
| Feedback | Toast 四种状态、Notification、确定/不确定进度条、事件日志 |
| Data | 交替行列表、卡片网格、带日志的计数器 |

顶栏右侧有一个**实时帧率计**：它统计每秒构建的帧数，帧率 ≥55 显示绿色、≥30 黄色、否则红色。
静止时界面不重绘，数字会归零——**滑动屏幕即可看到该页面的真实帧率**。

### 前置要求

- Rust 1.85 或更高（`Cargo.toml` 使用 edition 2024），并安装
  `aarch64-linux-android` / `x86_64-linux-android` target
- Android SDK + NDK
- JDK 17 或更高
- `gpuiforge`：`cargo install --path <gpuiforge 仓库>`

### 第一步：准备 GPUI 依赖

`Cargo.toml` 通过**本地路径**依赖 `gpui-fusion` 的 crate。clone 之后必须把路径改成你机器上的实际位置：

```powershell
# 先把 gpui-fusion 放到任意位置
git clone https://github.com/cradiy/gpui-fusion.git D:\Projects\gpui\gpui-fusion

# 再把本项目的路径依赖指向它（PowerShell）
$root = 'D:\Projects\gpui\gpui-fusion'
(Get-Content Cargo.toml) -replace 'D:/Projects/gpui/gpui-fusion', ($root -replace '\\','/') | Set-Content Cargo.toml
```

```bash
# 或者用 bash
root=/path/to/gpui-fusion
sed -i "s#D:/Projects/gpui/gpui-fusion#$root#g" Cargo.toml
```

`gpuiforge.json` 里的 `application-id` 也可以换成你自己的包名。

### 第二步：构建并运行

```powershell
# 检查工具链（JDK / SDK / NDK / 设备）
gpuiforge doctor

# 列出已连接设备及其 ABI
gpuiforge devices

# 构建并安装到设备（推荐 release，原因见下）
gpuiforge run android --release

# 只在宿主机构建，不安装
gpuiforge build android --release

# 多个设备时显式指定
gpuiforge run android --release --device <serial>

# 只构建某一个 ABI（默认跟随设备）
gpuiforge build android --release --abi arm64-v8a
gpuiforge build android --release --abi x86_64
```

`gpuiforge` 会在 `target/gpuiforge/android/` 下生成 Gradle 工程，输出 APK 到
`target/gpuiforge/android/app/build/outputs/apk/<debug|release>/`。

### 关于 debug 与 release

**请用 `--release` 构建。**

上游 `gpui-fusion` 的 debug 构建会把 wgpu 的 `DEBUG` 标志打开，从而启用
`VK_EXT_debug_utils`，在部分 Android GPU 驱动（以及模拟器的转译驱动）上会让进程在创建
第一个 GPU adapter 时直接 `SIGSEGV`：

```
F DEBUG : #00 pc ... /vendor/lib64/hw/vulkan.kona.so
                 (vk_common_SetDebugUtilsObjectNameEXT+291)
F libc  : Fatal signal 11 (SIGSEGV), fault addr 0x0
```

release 构建不会设置该标志，因此正常。如果你需要在 debug 下调试，可以把这一处修复
（[cradiy/gpui-fusion#9](https://github.com/cradiy/gpui-fusion/pull/9)）应用到你的
`gpui-fusion` 副本，或自行在 Android 上去掉该标志。

### 前提与已知限制

- 只在 **Android** 上验证过。代码里有 `#[cfg(target_os = "android")]` 分支，
  `gpui_platform` 也支持桌面，但本项目没有针对桌面做过适配。
- 界面按手机竖屏设计，横屏下内容会被拉伸或裁切。
- 带玻璃效果的页面（Glass）在 120Hz 屏幕上通常比其它页面更容易掉帧：
  它同时存在三个 backdrop 模糊面板，每个面板都需要独立的渲染 pass。
  60Hz 下实测可以满帧。

### 目录结构

```
src/main.rs       全部界面与状态（七个页面 + 帧率计）
Cargo.toml        依赖与编译档位（debug 下也对热点 crate 开启优化）
gpuiforge.json    Android 构建/运行步骤、包名、ABI
```

### 给 AI 助手的测试指引

想自动验证这个应用，可按下面顺序执行，每一步都可判定成败：

```powershell
# 1. 环境与设备
gpuiforge doctor
gpuiforge devices                     # 记住输出里的 serial 和 ABI

# 2. 构建安装（用设备对应的 ABI，避免走 ARM 转译）
gpuiforge build android --release --abi <arm64-v8a|x86_64>
gpuiforge run android --release --device <serial>

# 3. 启动是否成功（进程存在 + 选中了 GPU + 无崩溃）
adb -s <serial> shell pidof com.example.myapp
adb -s <serial> logcat -d | Select-String "Selected GPU adapter|FATAL|SIGSEGV"

# 4. 画面是否渲染（有内容则颜色数明显大于一个纯色值）
adb -s <serial> exec-out screencap -p > shot.png

# 5. 交互是否生效（切换底部页签后截图应有明显差异）
adb -s <serial> shell input tap <x> <y>
```

阅读帧率时注意两点：

- 顶栏帧率计只在重绘时更新，必须**模拟滑动**（`adb shell input swipe`）才能读到该页面的数值。
- `dumpsys SurfaceFlinger --latency <layer>` 在多数设备可用，但 Android 15 的部分
  ROM（例如 MuMu 模拟器）只返回刷新周期、不返回逐帧时间戳，此时以界面上的帧率计为准。

---

## English

### What this is

A GPUI component gallery that runs on Android. It gathers the widgets and visual effects
available in [`gpui-fusion`](https://github.com/cradiy/gpui-fusion) into seven pages so the
touch behaviour, rendering, and frame rate can be judged on a real device.

This repository does not contain GPUI itself. It is only the application layer
(~1600 lines in `src/main.rs`), built and installed through
[`gpuiforge`](https://github.com/cradiy/gpuiforge), which generates the Android host project.

### Pages

| Page | Contents |
| --- | --- |
| Basics | Type scale, palette, radii and shadows, buttons, badges |
| Inputs | Text, password, multiline, and numeric fields (with step and range) |
| Controls | Checkbox, switch, radio group, segmented control, slider |
| Glass | Liquid glass segmented control (horizontal and vertical), frosted-glass backdrop blur, transparency and motion toggles |
| Overlays | Modal, bottom sheet, elevated card |
| Feedback | Toasts in four states, notification, determinate and indeterminate progress, event log |
| Data | Alternating list rows, card grid, counter with log |

The header carries a live frame counter: it counts frames built per second and colours itself
green at ≥55, yellow at ≥30, red below. A static screen does not redraw, so the number falls to
zero — **scroll the page to read that page's real frame rate**.

### Requirements

- Rust 1.85 or newer (the manifest uses edition 2024) with the
  `aarch64-linux-android` / `x86_64-linux-android` targets installed
- Android SDK and NDK
- JDK 17+
- `gpuiforge`: `cargo install --path <gpuiforge checkout>`

### Step 1: point the GPUI dependency at your checkout

`Cargo.toml` depends on `gpui-fusion` through a **local path**, so it has to be rewritten after
cloning:

```powershell
git clone https://github.com/cradiy/gpui-fusion.git D:\Projects\gpui\gpui-fusion
$root = 'D:\Projects\gpui\gpui-fusion'
(Get-Content Cargo.toml) -replace 'D:/Projects/gpui/gpui-fusion', ($root -replace '\\','/') | Set-Content Cargo.toml
```

```bash
git clone https://github.com/cradiy/gpui-fusion.git /path/to/gpui-fusion
sed -i "s#D:/Projects/gpui/gpui-fusion#/path/to/gpui-fusion#g" Cargo.toml
```

You may also change `application-id` in `gpuiforge.json` to your own package name.

### Step 2: build and run

```powershell
gpuiforge doctor                     # check JDK / SDK / NDK / devices
gpuiforge devices                    # list devices with their ABIs
gpuiforge run android --release      # build, install, launch
gpuiforge build android --release    # build only
gpuiforge run android --release --device <serial>   # pick a device explicitly
gpuiforge build android --release --abi arm64-v8a   # single ABI
gpuiforge build android --release --abi x86_64
```

The generated Gradle project lands in `target/gpuiforge/android/`, and the APK in
`target/gpuiforge/android/app/build/outputs/apk/<debug|release>/`.

### Use release, not debug

Upstream `gpui-fusion` enables wgpu's `DEBUG` instance flag in debug builds, which adds
`VK_EXT_debug_utils` and names every Vulkan object. Several Android GPU drivers — including the
translated driver used by emulators — crash inside that entry point while the first adapter is
being created:

```
F DEBUG : #00 pc ... /vendor/lib64/hw/vulkan.kona.so
                 (vk_common_SetDebugUtilsObjectNameEXT+291)
F libc  : Fatal signal 11 (SIGSEGV), fault addr 0x0
```

Release builds do not set the flag, so they run normally. To debug on device, apply
[cradiy/gpui-fusion#9](https://github.com/cradiy/gpui-fusion/pull/9) to your checkout, or drop
the flag yourself on Android.

### Scope and known limits

- Verified on **Android** only. The code has `#[cfg(target_os = "android")]` branches and
  `gpui_platform` also targets desktop, but this project has not been adapted for desktop.
- Laid out for a phone in portrait; landscape stretches or clips the content.
- The Glass page is the first to drop frames on a 120Hz panel: three backdrop-blurred panels
  are on screen at once and each needs its own render passes. At 60Hz it holds a full frame.

### Layout

```
src/main.rs       all UI and state (seven pages plus the frame counter)
Cargo.toml        dependencies and build profiles (hot crates stay optimised in debug)
gpuiforge.json    Android build/run steps, package id, ABIs
```

### Notes for an AI agent testing this

Each step below has a pass/fail signal:

```powershell
gpuiforge doctor                                 # toolchain present
gpuiforge devices                                # note the serial and ABI

# Build for the device's own ABI so nothing runs through ARM translation
gpuiforge build android --release --abi <arm64-v8a|x86_64>
gpuiforge run android --release --device <serial>

adb -s <serial> shell pidof com.example.myapp    # process alive => launched
adb -s <serial> logcat -d | Select-String "Selected GPU adapter|FATAL|SIGSEGV"
adb -s <serial> exec-out screencap -p > shot.png # non-trivial colour count => rendered

adb -s <serial> shell input tap <x> <y>          # screenshot should change visibly
```

Two things to know when reading frame rate:

- The header counter only advances while frames are produced, so drive it with
  `adb shell input swipe` rather than reading a static screen.
- `dumpsys SurfaceFlinger --latency <layer>` works on most devices, but some Android 15 builds
  (MuMu emulators among them) return only the refresh period without per-frame timestamps. In
  that case trust the in-app counter.

## Licence

MIT. GPUI and `gpui-fusion` carry their own licences; see those repositories.
