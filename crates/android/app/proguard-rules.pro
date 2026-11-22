# btleplug Java classes (accessed only via JNI from the native library).
-keep class com.nonpolynomial.** { *; }
-keep class io.github.gedgygedgy.** { *; }

# Permission activity: launched by Android and reached from JNI.
-keep class com.openwatch.OpenWatchActivity { *; }
-keepclasseswithmembernames class * {
    native <methods>;
}
