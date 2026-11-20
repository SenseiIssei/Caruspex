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
- **Bluetooth LE adapters** (Vgate iCar Pro BLE, vLinker, OBDLink CX and MX+
  in BLE mode, Veepeak BLE, other FFF0 and FFE0 dongles): no pairing, the
  search of the Bluetooth choice finds them, with signal strength and the
  serial service they advertise. Android 12 and newer need the same "nearby
  devices" as classic Bluetooth (the scan is declared `neverForLocation`).
  Android 7 to 11 only report LE devices when the app may use the location
  and the location is switched on; the page asks for it separately, paired
  classic adapters keep working without it.
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

### How Bluetooth LE works on the phone

btleplug (the desktop's Bluetooth LE library) would need its own Java
classes in the APK and a JNI start-up from the activity. Instead the Kotlin
plugin uses Android's `BluetoothLeScanner` and `BluetoothGatt` directly, the
same way it opens classic Bluetooth, and the Rust side keeps every decision:

1. `bleScan` listens for the given time and returns address, name, RSSI and
   the advertised service UUIDs. Nothing is sent.
   `ble_profile::advertised_profile` and `scan_score` turn that into the list
   (a vLinker or OBDLink service counts like an adapter name; FFF0, FFE0 and
   Nordic UART modules are also in lamps and toys and only lift a device a
   little).
2. `bleConnect` connects (`TRANSPORT_LE`, one more try after a quick error
   133), discovers the services and returns them with their property bits.
   Still nothing is written.
3. `ble_gatt::plan_channel` picks the channel with the same profiles as the
   desktop (`ble_profile.rs`) and refuses every device that is not
   recognised as an OBD adapter by name or by a vendor service only adapters
   use. A refused device is disconnected without a single write.
4. Only then `bleSetup` asks for an MTU of 517, writes the CCC descriptor
   (notifications, or indications when the characteristic only indicates)
   and allows writes on that link.
5. `ble_gatt::GattTransport` is the `Transport` the ELM327 driver reads: it
   cuts every write to the agreed MTU minus 3 bytes, the Kotlin side waits
   for each write's callback before the next one, and notifications are
   collected in the same inbox as the bytes of a serial port.

The worker's connect job is the same as on the desktop (`Job::OpenBle`): LE
first, for a dual-mode adapter then classic as the second try. The logic of
steps 3 and 5 is tested on the desktop with fake links
(`crates/caruspex-link/src/ble_gatt.rs`, `tests/ble_gatt.rs`, the driver over
notifications of 20 bytes); the Kotlin side has not run on a phone yet.

## Not yet

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
