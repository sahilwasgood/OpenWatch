//! Android host for OpenWatch.
//!
//! The app is a `cdylib` loaded by `android.app.NativeActivity` (subclassed by
//! `OpenWatchActivity`). Slint 1.19 uses android-activity, so the exported
//! `android_main` initialises the Slint backend and then drives the shared UI
//! through a live `DeviceLink`.
//!
//! BLE permissions are requested from Java before btleplug creates its global
//! adapter; the native result callback gates adapter creation.

use openwatch_core::ble::device_key::DeviceKey;
use openwatch_ui::{connect_watch, run_live, AppError, UiOptions};

/// Initialise the watch connection and run the UI.
///
/// On Android, permissions and JNI must be initialised first (see
/// `android_main`), before any BLE manager is created.
pub fn launch(key: DeviceKey) -> Result<(), AppError> {
    run_live(connect_watch(key)?, UiOptions::default())
}

#[cfg(target_os = "android")]
mod android {
    use std::ffi::c_void;
    use std::sync::{mpsc, Mutex};

    use jni::objects::{JClass, JObject, JValue};
    use jni::sys::jboolean;
    use jni::{jni_sig, jni_str, EnvUnowned, JavaVM, NativeMethod};
    use openwatch_core::ble::device_key::{DeviceKey, WATCH_ADVERTISED_NAME};
    use slint::android::AndroidApp;

    /// Set by `android_main` before Java is asked for permissions. The Java
    /// callback takes the sender so a repeated grant cannot send twice.
    static PERMISSION_TX: Mutex<Option<mpsc::Sender<bool>>> = Mutex::new(None);

    /// Registered as `OpenWatchActivity.nativeBlePermissionResult`.
    extern "C" fn on_ble_permission_result<'local>(
        _env: EnvUnowned<'local>,
        _class: JClass<'local>,
        granted: jboolean,
    ) {
        if let Ok(mut slot) = PERMISSION_TX.lock() {
            if let Some(tx) = slot.take() {
                let _ = tx.send(granted != 0);
            }
        }
    }

    /// Register the native callback, then ask the activity for the runtime
    /// permissions. Java calls the callback either immediately (already
    /// granted) or from `onRequestPermissionsResult`.
    fn request_permissions(
        vm: &JavaVM,
        activity_raw: jni::sys::jobject,
    ) -> Result<(), Box<dyn std::error::Error>> {
        vm.attach_current_thread(|env| {
            let class = env.find_class(jni_str!("com/openwatch/OpenWatchActivity"))?;
            unsafe {
                env.register_native_methods(
                    &class,
                    &[NativeMethod::from_raw_parts(
                        jni_str!("nativeBlePermissionResult"),
                        jni_str!("(Z)V"),
                        on_ble_permission_result as *mut c_void,
                    )],
                )?;
            }
            let activity = unsafe { JObject::from_raw(env, activity_raw) };
            env.call_static_method(
                &class,
                jni_str!("requestBlePermissions"),
                jni_sig!("(Landroid/app/Activity;)V"),
                &[JValue::Object(&activity)],
            )?;
            Ok(())
        })
    }

    /// Only called after the grant callback, so btleplug's `Adapter::new`
    /// never runs without permission. The class loader is set by Java first.
    fn init_btleplug(
        vm: &JavaVM,
        activity_raw: jni::sys::jobject,
    ) -> Result<(), Box<dyn std::error::Error>> {
        vm.attach_current_thread(|env| {
            let class = env.find_class(jni_str!("com/openwatch/OpenWatchActivity"))?;
            let activity = unsafe { JObject::from_raw(env, activity_raw) };
            env.call_static_method(
                &class,
                jni_str!("prepareNativeThread"),
                jni_sig!("(Landroid/app/Activity;)V"),
                &[JValue::Object(&activity)],
            )?;
            btleplug::platform::init(env)?;
            Ok(())
        })
    }

    #[unsafe(no_mangle)]
    pub fn android_main(app: AndroidApp) {
        let (tx, rx) = mpsc::channel();
        *PERMISSION_TX.lock().expect("permission slot") = Some(tx);

        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
        let activity_raw = app.activity_as_ptr() as jni::sys::jobject;

        if let Err(error) = request_permissions(&vm, activity_raw) {
            log::error!("could not request BLE permissions: {error}");
            return;
        }
        // Android shows the dialog on its main thread; this native thread waits
        // for the grant callback before any adapter is created.
        let granted = rx.recv().unwrap_or(false);

        slint::android::init(app).expect("Slint Android backend must initialise");

        if !granted {
            log::error!("BLE permissions denied; watch connection unavailable");
            return;
        }
        if let Err(error) = init_btleplug(&vm, activity_raw) {
            log::error!("btleplug init failed: {error}");
            return;
        }

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
            "android:name=\".OpenWatchActivity\"",
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
