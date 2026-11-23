# OpenWatch Android host

`openwatch-android` is a `cdylib` loaded by `android.app.NativeActivity`. The
Rust entrypoint `android_main` initialises Slint's Android backend, initialises
btleplug's JNI bridge, then hands a live `ConnectionLink` to the shared UI in
`openwatch-ui`. The generated Slint window never leaves that crate.

## Layout

- `manifest/AndroidManifest.xml` — canonical manifest (BLE permissions, minSdk 26, NativeActivity with `android.app.lib_name = openwatch_android`).
- `app/build.gradle` — AGP module; points at the manifest above and at `app/src/main/jniLibs`.
- `app/proguard-rules.pro` — R8 keep rules for btleplug's JNI-only Java classes.
- `app/src/main/java/com/openwatch/OpenWatchActivity.java` — requests BLE
  runtime permissions and gates the native adapter creation.
- `src/lib.rs` — `android_main`, the permission callback and `launch`.

## Prerequisites

- Android SDK (API 34) and NDK **r30+** (Slint's Skia prebuilts need it; older
  NDKs fail on `__ndk1`). Set `ANDROID_HOME` and `ANDROID_NDK_ROOT`.
- Host `clang`, JDK 17+, Gradle 8.9+ with AGP 8.7.3.
- `cargo install cargo-ndk`.

## Build the AAB

```sh
# 1. btleplug's Java/JNI support classes (ships inside the btleplug crate).
JAVA_DIR=$(find ~/.cargo/registry/src -maxdepth 5 -type d \
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

btleplug does not request runtime permissions. `OpenWatchActivity` (a
`NativeActivity` subclass) asks for `BLUETOOTH_SCAN`/`BLUETOOTH_CONNECT` on
API 31+ and `ACCESS_FINE_LOCATION` on older APIs. Its result is forwarded to a
native callback registered before the request, so btleplug's global adapter is
created only after the grant. Denial logs an error and stops without touching
BLE — no empty UI is presented as success. The Java/native permission handshake
can only be verified on a device; CI just compiles and packages it.

## CI

`.github/workflows/android.yml` runs on tags `app-v*`: it installs the NDK,
builds the JNI AAR and four native libraries, assembles the AAB and uploads it
as an artifact.
