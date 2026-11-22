# Platform and accessibility de-risking (2026-11-22)

Two research lanes run in parallel with Task 8. Findings that change later tasks.

## Slint 1.19 + Android (Task 12)
- Slint uses the `android-activity` crate, not winit, via feature `backend-android-activity-06`.
  The app is a `cdylib` exporting `#[unsafe(no_mangle)] fn android_main(app: slint::android::AndroidApp)`,
  calls `slint::android::init(app)`, then runs the window. Activity must be
  `android.app.NativeActivity` with `android.app.lib_name` set. `minSdk = 26`.
- Skia prebuilts need **NDK r30+** (older NDKs crash on `__ndk1`); alternative `FORCE_SKIA_BUILD=1`.
  Host `clang` required. Target `aarch64-linux-android`.
- AAB path: `cargo ndk -t arm64-v8a -o app/src/main/jniLibs build --release --lib`, then Gradle
  `./gradlew bundleRelease`. `cargo-apk` only makes APK; Tauri is a web stack and a poor fit.
- `.slint` in a library: `build.rs` calls `slint_build::compile_with_config("ui/ui.slint", ...)`;
  `slint::include_modules!()` in the library. **Generated Slint types cannot be imported across
  crates** (open issue #7060) — `crates/ui` must expose Rust wrapper APIs that frontends call.
- BLE permissions: API 31+ `BLUETOOTH_SCAN` (with `usesPermissionFlags="neverForLocation"`) and
  `BLUETOOTH_CONNECT`; cap legacy `BLUETOOTH`/`BLUETOOTH_ADMIN`/`ACCESS_FINE_LOCATION` at
  `maxSdkVersion="30"`. btleplug does **not** request permissions — request before `Adapter::new`.
- JNI: call `btleplug::platform::init` on a thread whose class loader sees app classes.
  R8 keep rules: `com.nonpolynomial.**` and `io.github.gedgygedgy.**`.
- Sources: Slint Android guide, docs.rs slint-build, Android BLE permissions docs, btleplug README.

## Accessibility verification
- WCAG 2.2 AA: 4.5:1 normal text, 3:1 large text and UI components. Measure the **composited**
  glass over black and white plus real worst-case backdrops; declared tint values are not enough.
- Headless check: `slint-viewer --screenshot out.png ui.slint` with `--load-data` JSON, then sample
  pixels. Use a Skia backend — software rendering won't show `backdrop-blur`.
- Slint 1.19 reduced-motion is honored only on Windows and Android; macOS/Linux ignore it.
  **No reduced-transparency or forced-colors detection** — the app must ship its own opaque/no-blur
  toggle (already in the design as "solid surfaces").
- Custom controls need `accessible-role`, label and actions. Test trees with Accessibility Insights
  (Windows), Accessibility Inspector (macOS), Accerciser/pyatspi (Linux). Headless alternative:
  `i-slint-backend-testing` or Slint's MCP server with `SLINT_BACKEND=headless-skia`.
- Not supported in 1.19: reliable text-field screen-reader behavior, menu roles, web/Wasm
  screen readers, documented Android TalkBack support. Manual device check required for TalkBack.
- Sources: WCAG 2.2, Slint 1.19 animations reference, Slint best-practices and tooling docs,
  Slint GitHub issues #2895/#8732/#14989.
