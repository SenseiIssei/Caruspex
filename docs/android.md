# Caruspex on Android

The Android app is the same Tauri app as on the desktop: the Rust core in
`app/src-tauri` built as a library (`libcaruspex_app_lib.so`), the same web
interface, and a small Kotlin plugin for what only Android's Java side can
open. The Android Studio project lives in `app/src-tauri/gen/android` and is
kept in git, as Tauri recommends.

## What works

- **WLAN adapters** (ELM327 WiFi): plain TCP like on the desktop. While
  connected, the app binds its sockets to the WLAN, because a WLAN adapter
  has no internet and Android would otherwise send them out over mobile data.
- **Demo car and recordings**: like on the desktop. A recording picked in the
  file dialog is copied into the app's cache and replayed from there.
- **Classic Bluetooth** (serial port profile, most cheap ELM327 dongles): the
  adapter is paired once in the Android settings (PIN usually 1234 or 0000),
  then it is in the list of the Bluetooth choice. Android 12 and newer ask for
  "nearby devices"; the connection page explains why before the system asks.
  Older versions need no question for paired devices.
- **USB adapters on an OTG cable** (FTDI, CH340, CP210x, PL2303, CDC) through
  [usb-serial-for-android](https://github.com/mik3y/usb-serial-for-android)
  (MIT). Android asks once per adapter whether Caruspex may use it.
- **Phone layout** from 360 px width: the sidebar becomes a menu, tables
  scroll, controls are large enough for fingers, the X-ray view turns with
  one finger and zooms with two.
- **Files**: exports and backups go through the system file dialog; the
  report is saved as an HTML page (a phone has no hidden window to print a
  PDF from), which every browser there shows and prints. Log and recording
  files go into the app's own documents folder
  (`Android/data/dev.senseiissei.caruspex/files/Documents`, visible from a
  computer over USB).

## Not yet

- **Bluetooth LE adapters** (Vgate iCar Pro BLE, vLinker, OBDLink CX). btleplug
  compiles for Android but needs its Java side (droidplug) in the APK and a
  JNI start-up from the activity; without a phone to test on that was left
  out. The Bluetooth choice says so.
- **KKL cables** (K-line bit timing over USB) and **CAN interfaces**: the
  choices are hidden on the phone.
- The app has no own updater on Android; it shows that a newer version exists
  and opens the release page.

## Building

Needed: the Android SDK with the NDK, JDK 17 or 21 (Gradle does not run on
newer ones yet), Node and the Rust targets:

    rustup target add aarch64-linux-android x86_64-linux-android
    export ANDROID_HOME=~/Android/Sdk NDK_HOME=$ANDROID_HOME/ndk/<version>
    cd app
    npx tauri android build --debug --target aarch64 --apk   # phone
    npx tauri android build --debug --target x86_64 --apk    # emulator
    npx tauri android build --target aarch64 --apk           # release

The APK lands in
`app/src-tauri/gen/android/app/build/outputs/apk/<arch>/<debug|release>/`.

On Windows the Tauri CLI links the built library into `jniLibs` with a
symbolic link, which Windows only allows in developer mode. Without it, copy
`target/aarch64-linux-android/debug/libcaruspex_app_lib.so` to
`app/src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a/` yourself after
the failed link step and run Gradle without its Rust task:

    cd app/src-tauri/gen/android
    gradlew assembleArm64Debug -x rustBuildArm64Debug -PtargetList=aarch64 -ParchList=arm64 -PabiList=arm64-v8a

## CI and release signing

The `Android` job of `.github/workflows/build.yml` builds a release APK for
arm64 on every push. It is not a required check. Without signing secrets the
APK stays unsigned and is only kept as the build artifact `android-apk`.

On a version tag, when these repository secrets exist, the APK is signed and
attached to the GitHub release as `Caruspex-<version>-android-arm64.apk`:

| Secret | Content |
|---|---|
| `CARUSPEX_ANDROID_KEYSTORE` | the keystore file, base64 (`base64 -w0 caruspex-release.jks`) |
| `CARUSPEX_ANDROID_KEYSTORE_PASSWORD` | the password of the keystore |
| `CARUSPEX_ANDROID_KEY_ALIAS` | the alias of the key in it |
| `CARUSPEX_ANDROID_KEY_PASSWORD` | the password of the key |

The release key is made once by the owner and never committed, for example:

    keytool -genkeypair -v -keystore caruspex-release.jks -alias caruspex \
      -keyalg RSA -keysize 4096 -validity 10000

Keep the keystore and its passwords safe: every later version has to be
signed with the same key, or phones refuse to install it as an update.

Locally the same variables sign a release build:
`CARUSPEX_ANDROID_KEYSTORE_FILE` (path to the keystore) plus the three
others above.
