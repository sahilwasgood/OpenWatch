//! Android host for OpenWatch.
//!
//! The app is a `cdylib` loaded by `android.app.NativeActivity`. Slint 1.19
//! uses android-activity, so the exported `android_main` initialises the Slint
//! backend and then drives the shared UI through a live `DeviceLink`.

use openwatch_core::ble::device_key::DeviceKey;
use openwatch_ui::{connect_watch, run_live, AppError, UiOptions};

/// Initialise the watch connection and run the UI.
///
/// On Android, JNI must be initialised first (see `android_main`), before any
/// BLE manager is created.
pub fn launch(key: DeviceKey) -> Result<(), AppError> {
    run_live(connect_watch(key)?, UiOptions::default())
}

#[cfg(target_os = "android")]
mod android {
    use openwatch_core::ble::device_key::{DeviceKey, WATCH_ADVERTISED_NAME};
    use slint::android::AndroidApp;

    /// btleplug's droidplug needs a JNI `Env` whose class loader sees the app
    /// classes. `android_main` runs on that thread for a NativeActivity.
    fn init_btleplug(app: &AndroidApp) {
        let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
        let result = vm.attach_current_thread(|env| -> Result<(), Box<dyn std::error::Error>> {
            btleplug::platform::init(env)?;
            Ok(())
        });
        if let Err(error) = result {
            log::warn!("btleplug JNI init failed; reads will stay empty: {error}");
        }
    }

    #[unsafe(no_mangle)]
    pub fn android_main(app: AndroidApp) {
        init_btleplug(&app);
        slint::android::init(app).expect("Slint Android backend must initialise");
        let key = DeviceKey::AdvertisedName(WATCH_ADVERTISED_NAME.to_string());
        if let Err(error) = super::launch(key) {
            log::error!("OpenWatch stopped: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    const MANIFEST: &str = include_str!("../manifest/AndroidManifest.xml");

    #[test]
    fn manifest_declares_the_required_permissions_and_abi() {
        for token in [
            "android.app.NativeActivity",
            "android.app.lib_name",
            "android:value=\"openwatch_android\"",
            "BLUETOOTH_SCAN",
            "neverForLocation",
            "BLUETOOTH_CONNECT",
            "BLUETOOTH_ADMIN",
            "ACCESS_FINE_LOCATION",
            "android.permission.INTERNET",
            "android:minSdkVersion=\"26\"",
        ] {
            assert!(MANIFEST.contains(token), "manifest lost {token}");
        }
    }
}
