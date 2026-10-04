# Cussy on iPhone, iPad, and Android

The mobile apps let you edit a program, open a UTF-8 `.cussy` file, and run it
locally. Output and source diagnostics appear below the editor. No server or
account is required. The same Rust interpreter runs inside both apps through a
small C interface; there is no external compiler, subprocess, or JIT on the phone.

## Install

Download the mobile assets from [GitHub Releases](https://github.com/cussylang/cussylang/releases/latest).

| Platform | Download | Installation |
| --- | --- | --- |
| Android 8.0+ (ARM64 phones and x86_64 emulators) | `cussy-0.3.0-android.apk` | Open the APK and allow installation from the browser/file manager when Android asks. |
| iPhone and iPad, iOS/iPadOS 16+ | `cussy-0.3.0-ios-unsigned.ipa` | This app bundle is **unsigned**. Sign it with your own Apple development identity/provisioning profile, or build the Xcode project below. It cannot install by simply opening the download. |
| Developers embedding Cussy in an Apple app | `cussy-0.3.0-ios-runtime.zip` | Add `CussyRuntime.xcframework` to your Xcode target and import `CussyRuntime`. |

The Android download is a **debug-signed development APK**, not a Play Store
release. CI uses an ephemeral debug key, so later APKs may require uninstalling
the previous one; save your source before doing that. The iOS package is not an
App Store or TestFlight release. All downloads have entries in `SHA256SUMS`.

The apps run the interpreter, including arrays, records, checked pointers, math,
strings, loops, and embedded standard modules. The desktop `cussy compile`
command remains the way to generate optimized native executables and assembly.
Its desktop performance table is not a mobile performance measurement.

## Mobile execution behavior

- Each run starts with fresh state and executes away from the UI thread.
- Source must be valid UTF-8 and at most 1 MiB. The runtime also rejects programs
  that exceed 8,192 tokens total or 1,024 tokens between statement/block boundaries
  before recursive checking/execution. Comments and string contents do not count
  as separate tokens.
- Runs have at most 5,000,000 interpreter steps, a 128-call recursion limit, and
  the existing 8 MiB output limit. Fuel is a work limit, not a wall-clock deadline.
- `graph math;` and the other embedded standard imports work. Imports from other
  source files are unavailable in the single-file mobile editor.
- Programs cannot read or write host files, export SVG plots, access environment
  variables or stdin, sleep, or call native FFI. These operations produce a
  `CAPABILITY` diagnostic. Opening a file in the app imports its source into the
  editor; it does not grant that program filesystem access.

The runtime executes inside the app process. These limits and disabled host
operations are not an OS process or memory sandbox for hostile programs.

## Build Android

Install Rust through rustup, Java 17, and Android Studio or the Android command
line tools. Point `ANDROID_HOME` at your SDK. The checked-in Gradle wrapper pins
the Gradle distribution and verifies its checksum; a separate Gradle installation
is unnecessary.

From the repository root:

```sh
rustup toolchain install 1.88.0 --profile minimal
sdkmanager 'platforms;android-36' 'build-tools;35.0.0' 'ndk;27.2.12479018' 'cmake;3.22.1'
python3 scripts/build_mobile.py android
cd mobile/android
./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

The build helper adds the required Rust target libraries and compiles optimized
ARM64 and x86_64 static libraries. It writes them into the app's `jniLibs` input
directories; CMake links them into the JNI library. Use `--abis arm64-v8a` to
build only the device ABI and pair that with `./gradlew -PcussyAbis=arm64-v8a assembleDebug`.
Use `--ndk PATH` to select the installed NDK directory.
The Gradle project and CMake use 16 KiB alignment for the packaged native library.

For an emulator/device test, build both default ABIs, start an Android 8.0+
emulator or connect a device, and run `./gradlew connectedDebugAndroidTest`.
The tests execute Cussy through the actual Java → JNI → Rust interface.

## Build iOS and iPadOS

Use macOS with **full Xcode**, its iOS SDKs, and an installed iOS simulator.
Apple's standalone Command Line Tools do not include these SDKs. Select your
Xcode installation in Xcode's Locations settings (or set `DEVELOPER_DIR`).

```sh
rustup toolchain install 1.88.0 --profile minimal
python3 scripts/build_mobile.py ios
open mobile/ios/Cussy.xcodeproj
```

Choose the **Cussy** scheme and an iPhone/iPad simulator, then Run. For your own
device, select your Apple team under Signing & Capabilities and choose a bundle
identifier registered to that team. Build and run with the device selected.
Apple's [device and simulator guide](https://developer.apple.com/documentation/xcode/running-your-app-on-simulated-or-physical-devices)
describes the signing and device setup.

The helper builds ARM64 iOS plus ARM64 and x86_64 simulator libraries, then creates
`mobile/build/CussyRuntime.xcframework`. Device and simulator libraries are kept
in separate slices. To use the downloaded runtime instead, unzip it into
`mobile/build/` so the project finds the same framework path.

Use Product → Test to execute the XCTest suite in a simulator. CI additionally
builds an unsigned device app. Simulator tests and device compilation do not
establish that the app has been installed or tested on a physical iPhone.

## Embed the runtime

The C interface is declared in [cussy_mobile.h](../mobile/include/cussy_mobile.h):

```c
char *cussy_mobile_run(const uint8_t *source, size_t source_len, uint64_t fuel);
void cussy_mobile_free(char *result);
uint32_t cussy_mobile_api_version(void);
```

Call `cussy_mobile_run` on a background thread. A fuel argument of zero selects
the default; values above the maximum are capped. Copy the returned UTF-8 JSON
and free that pointer exactly once with `cussy_mobile_free`, never with the host
language's allocator. The source buffer must remain readable for the call.
The API returns `{ "ok", "exit_code", "output", "diagnostic" }`; `ok` means
execution completed, and a nonzero program exit code remains visible separately.
On a runtime error, captured output before the error is retained. Handle a null
result as an internal failure. ABI contract version is currently 1.

The [Swift app](../mobile/ios) and [Android JNI bridge](../mobile/android) are
working examples. Android static libraries are built by the same helper; iOS
libraries come in the XCFramework. Each call has independent state and does not
change the host app's current directory, environment, or output streams.

SDK references: [Rust iOS targets](https://doc.rust-lang.org/rustc/platform-support/apple-ios.html),
[Rust Android targets](https://doc.rust-lang.org/rustc/platform-support/android.html),
and [Android NDK cross-compilation](https://developer.android.com/ndk/guides/other_build_systems).
