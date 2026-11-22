# OpenWatch Android host

`openwatch-android` is a `cdylib` loaded by `android.app.NativeActivity`. The
Rust entrypoint `android_main` initialises Slint's Android backend, initialises
btleplug's JNI bridge, then hands a live `ConnectionLink` to the shared UI in
`openwatch-ui`. The generated Slint window never leaves that crate.

## Layout

- `manifest/AndroidManifest.xml` — canonical manifest (BLE permissions, minSdk 26, NativeActivity with `android.app.lib_name = openwatch_android`).
- `app/build.gradle` — AGP module; points at the manifest above and at `app/src/main/jniLibs`.
- `app/proguard-rules.pro` — R8 keep rules for btleplug's JNI-only Java classes.
- `src/lib.rs` — `android_main` and `launch`.

## Prerequisites

- Android SDK (API 34) and NDK **r30+** (Slint's Skia prebuilts need it; older
  NDKs fail on `__ndk1`). Set `ANDROID_HOME` and `ANDROID_NDK_ROOT`.
- Host `clang`, JDK 17+, Gradle 8.9+ with AGP 8.7.3.
- `cargo install cargo-ndk`.

## Build the AAB

```sh
# 1. btleplug's Java/JNI support classes (ships inside the btleplug crate).
JAVA_DIR=$(find ~/.cargo/registry/src -maxdepth 2 -type d \
  -path '*/btleplug-0.13.*/src/droidplug/java' | head -1)
(cd "$JAVA_DIR" && ./gradlew assembleRelease)
mkdir -p libs
cp "$JAVA_DIR/build/outputs/aar/droidplug-release.aar" libs/

# 2. Rust native libraries, one per ABI.
for target in aarch64-linux-android armv7-linux-androideabi \
              x86_64-linux-android i686-linux-android; do
  cargo ndk --target "$target" -o app/src/main/jniLibs \
    build --release -p openwatch-android --lib
done

# 3. Package.
gradle bundleRelease
# -> app/build/outputs/bundle/release/app-release.aab
```

`FORCE_SKIA_BUILD=1` builds Skia from source instead of using the prebuilts if
an older NDK must be used.

## Permissions

API 31+ uses `BLUETOOTH_SCAN` (`neverForLocation`) and `BLUETOOTH_CONNECT`; the
legacy Bluetooth and location permissions are capped at `maxSdkVersion=30`.
`INTERNET` is only consumed by the opt-in AI frame explainer.

btleplug does not request runtime permissions. On API 31+ the app must obtain
`BLUETOOTH_SCAN`/`BLUETOOTH_CONNECT` before the first `Adapter` is created; the
manifest declares them, but a runtime request UI is not part of this crate yet.
Without it the first scan is denied and the surfaces stay empty.

## CI

`.github/workflows/android.yml` runs on tags `app-v*`: it installs the NDK,
builds the JNI AAR and four native libraries, assembles the AAB and uploads it
as an artifact.
