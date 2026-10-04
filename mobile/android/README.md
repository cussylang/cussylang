# Cussy for Android

A native Android editor and runner for `.cussy` programs. Open and save UTF-8 files through the system document picker, edit a draft, and run it on the device. Drafts are stored in the app's private preferences and survive rotation and normal relaunch. Program output and diagnostics appear below the editor. The app has no network or storage permission; document access is granted through the picker.

The JNI library links the shared Rust interpreter, including its arrays, pointers, objects, and embedded standard modules. Host filesystem access, native FFI, environment access, stdin and sleep are disabled by the mobile runtime. Source is limited to 1 MiB and execution to 5,000,000 interpreter steps. The output view shows at most 65,536 characters, followed by any diagnostic. Execution happens on a single background executor; rotation retains the active session. This is an in-process interpreter, not an OS sandbox for hostile code.

## Build

Install JDK 17, Rust 1.88.0 or newer, and Android Studio's SDK packages: Android SDK 36, Build Tools 35.0.0, NDK 27.2.12479018, and CMake 3.22.1. Set `ANDROID_HOME` to the SDK directory and `JAVA_HOME` to JDK 17. From the repository root:

```sh
python3 scripts/build_mobile.py android
cd mobile/android
./gradlew assembleDebug testDebugUnitTest lintDebug
```

Open `mobile/android` in Android Studio for editing, installation, and debugging. The debug APK is `app/build/outputs/apk/debug/app-debug.apk`. It supports Android 8.0 / API 26 and newer on `arm64-v8a` and `x86_64`. A debug APK is a development build; distribution signing is a separate release configuration.

For one ABI, pair `python3 scripts/build_mobile.py android --abis arm64-v8a` with `./gradlew -PcussyAbis=arm64-v8a assembleDebug`.

The Rust build puts each ABI's `libcussy_mobile.a` in `app/src/main/jniLibs/<abi>/`. CMake links it into `libcussy_android.so` and includes `../include/cussy_mobile.h` from the shared mobile directory. Both native linking and APK packaging enable 16 KiB page alignment.

## Tests

```sh
./gradlew testDebugUnitTest
# With an API 26+ arm64-v8a or x86_64 emulator/device connected:
./gradlew connectedDebugAndroidTest
```

JVM tests cover strict UTF-8, embedded NUL, BOM handling, and exact import-size boundaries. Device tests execute the real Rust runtime through JNI, verify Unicode and nonzero exit status, arrays and pointers, diagnostics with prior output, the step budget and denied host access, and exercise Run plus activity recreation.

## Pinned tools and provenance

The [Android Gradle Plugin 8.13 compatibility table](https://developer.android.com/build/releases/agp-8-13-0-release-notes) specifies Gradle 8.13 and JDK 17 and supports SDK 36. The project pins AGP 8.13.2 and Gradle 8.13. Test-only AndroidX dependencies are pinned; the application uses platform widgets without a UI framework dependency. Native linker options follow Android's [16 KiB page-size guidance](https://developer.android.com/guide/practices/page-sizes).

`gradlew`, `gradlew.bat`, and `gradle/wrapper/gradle-wrapper.jar` are unmodified files from [Gradle's v8.13.0 tag](https://github.com/gradle/gradle/tree/v8.13.0). Their upstream [license](gradle/wrapper/LICENSE) is included. The JAR's SHA-256 was checked against [Gradle's published wrapper checksum](https://services.gradle.org/distributions/gradle-8.13-wrapper.jar.sha256), recorded in `gradle/wrapper/gradle-wrapper.jar.sha256`. `gradle-wrapper.properties` pins the distribution SHA-256 from [the official distribution checksum](https://services.gradle.org/distributions/gradle-8.13-bin.zip.sha256); the wrapper verifies it before using the download.
