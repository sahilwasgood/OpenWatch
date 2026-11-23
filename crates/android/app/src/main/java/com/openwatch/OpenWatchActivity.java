package com.openwatch;

import android.Manifest;
import android.app.Activity;
import android.app.NativeActivity;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Bundle;

/**
 * NativeActivity subclass that obtains the runtime BLE permissions before the
 * Rust entrypoint creates btleplug's global adapter. The native library
 * registers {@code nativeBlePermissionResult} before calling
 * {@code requestBlePermissions}, so both the already-granted and callback
 * paths report through the same native hook.
 */
public class OpenWatchActivity extends NativeActivity {
    private static final int BLE_REQUEST = 1;

    private static native void nativeBlePermissionResult(boolean granted);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        // Deliberately no request here: native registers its callback first.
    }

    /** Called from JNI once the native result callback is registered. */
    public static void requestBlePermissions(Activity activity) {
        String[] permissions;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            permissions = new String[] {
                Manifest.permission.BLUETOOTH_SCAN,
                Manifest.permission.BLUETOOTH_CONNECT
            };
        } else {
            // Legacy BLE scans need location on API 26-30.
            permissions = new String[] { Manifest.permission.ACCESS_FINE_LOCATION };
        }
        for (String permission : permissions) {
            if (activity.checkSelfPermission(permission) != PackageManager.PERMISSION_GRANTED) {
                activity.requestPermissions(permissions, BLE_REQUEST);
                return;
            }
        }
        nativeBlePermissionResult(true);
    }

    /** Called from JNI before btleplug init so its JNI classes are visible. */
    public static void prepareNativeThread(Activity activity) {
        Thread.currentThread().setContextClassLoader(activity.getClassLoader());
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, String[] permissions, int[] grantResults) {
        if (requestCode == BLE_REQUEST) {
            boolean granted = grantResults.length > 0;
            for (int result : grantResults) {
                if (result != PackageManager.PERMISSION_GRANTED) {
                    granted = false;
                    break;
                }
            }
            nativeBlePermissionResult(granted);
        } else {
            super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        }
    }
}
