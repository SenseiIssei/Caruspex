// Updates: one request to GitHub for latest.json of the newest release,
// without anything about the car or the user. The manifest names one
// package per platform with its minisign signature; a package is installed
// only after the signature matches the public key built into the app.
//
//   Windows  the Inno Setup installer, run silently; it keeps the install
//            mode and folder of the installation it updates
//   Linux    the AppImage file is replaced (a .deb install gets the page)
//   macOS    the .app bundle is replaced from the .app.tar.gz
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use caruspex_i18n::l;
use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;

use crate::texts::args;

/// The manifest of the newest release. GitHub answers this address with a
/// redirect to the file of the latest release.
const MANIFEST_URL: &str =
    "https://github.com/SenseiIssei/Caruspex/releases/latest/download/latest.json";
/// Packages are only loaded from the releases of this repository.
const DOWNLOAD_PREFIX: &str = "https://github.com/SenseiIssei/Caruspex/releases/download/";
const RELEASE_PAGE: &str = "https://github.com/SenseiIssei/Caruspex/releases/tag/v";
const MAC_DMG: &str =
    "https://github.com/SenseiIssei/Caruspex/releases/latest/download/Caruspex-macos.dmg";

/// The public half of the release key, as `tauri signer generate` writes it
/// (base64 of the minisign public key file). The private half only lives in
/// the CI secret CARUSPEX_UPDATER_KEY.
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEEyQURGM0UzMzE1ODhBQkMKUldTOGlsZ3g0L090b2dITHJBVUxNZnlvL2ZhTnh4a3htZmpaQU9VdXRzMjQrRHN0U3pyREFOM2sK";

/// For tests only: another manifest address (https or file://) and,
/// together with it, another public key. Without the address the key is
/// ignored, so a stray variable cannot weaken a normal installation.
const TEST_URL_ENV: &str = "CARUSPEX_UPDATE_URL";
const TEST_KEY_ENV: &str = "CARUSPEX_UPDATE_PUBKEY";

/// What the silent Inno Setup run gets. /CARUSPEXRESTART=1 starts the new
/// version afterwards (caruspex.iss): /RESTARTAPPLICATIONS only restarts
/// programs that registered with the Restart Manager, and Caruspex has
/// already quit by then.
pub const INNO_ARGUMENTS: [&str; 6] = [
    "/VERYSILENT",
    "/SUPPRESSMSGBOXES",
    "/NORESTART",
    "/CLOSEAPPLICATIONS",
    "/RESTARTAPPLICATIONS",
    "/CARUSPEXRESTART=1",
];

// --- Versions ----------------------------------------------------------------------------

/// The numeric segments at the start of a version text, like
/// QVersionNumber::fromString ("v1.2.3-beta" -> [1, 2, 3]).
pub fn version_segments(text: &str) -> Vec<u64> {
    let text = text.strip_prefix('v').unwrap_or(text);
    let mut out = Vec::new();
    for part in text.split('.') {
        let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            break;
        }
        out.push(digits.parse().unwrap_or(0));
        if digits.len() != part.len() {
            break;
        }
    }
    out
}

/// QVersionNumber ordering: segment by segment, and with an equal common
/// part the longer number is the newer one.
pub fn is_newer(candidate: &[u64], mine: &[u64]) -> bool {
    for (a, b) in candidate.iter().zip(mine) {
        if a != b {
            return a > b;
        }
    }
    candidate.len() > mine.len()
}

fn segments_text(v: &[u64]) -> String {
    v.iter().map(u64::to_string).collect::<Vec<_>>().join(".")
}

// --- The manifest ------------------------------------------------------------------------

/// latest.json of a release (the format of the Tauri updater): the version,
/// the change list with a German and an English part, the date and one
/// package per platform.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub pub_date: String,
    #[serde(default)]
    pub platforms: BTreeMap<String, Package>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Package {
    pub url: String,
    /// base64 of the minisign signature file, as `tauri signer sign` writes it.
    pub signature: String,
}

pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let manifest: Manifest = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if version_segments(&manifest.version).is_empty() {
        return Err(format!("no version in the manifest: {}", manifest.version));
    }
    Ok(manifest)
}

/// The manifest key of a system: windows-x86_64, linux-x86_64, darwin-aarch64.
pub fn platform_key(os: &str, arch: &str) -> Option<String> {
    let os = match os {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "darwin",
        _ => return None,
    };
    let arch = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return None,
    };
    Some(format!("{os}-{arch}"))
}

/// The German or the English part of the change list (the "## Deutsch" and
/// "## English" sections of changelog/<version>.md). Notes without those
/// headings come back whole.
pub fn notes_in(notes: &str, german: bool) -> String {
    let section = |name: &str| {
        let mut inside = false;
        let mut out = Vec::new();
        for line in notes.lines() {
            if let Some(heading) = line.strip_prefix("## ") {
                inside = heading.trim().eq_ignore_ascii_case(name);
                continue;
            }
            if inside {
                out.push(line);
            }
        }
        out.join("\n").trim().to_string()
    };
    if !notes.lines().any(|line| line.starts_with("## ")) {
        return notes.trim().to_string();
    }
    let wanted = section(if german { "Deutsch" } else { "English" });
    if wanted.is_empty() {
        section(if german { "English" } else { "Deutsch" })
    } else {
        wanted
    }
}

/// A newer release than this one, with the package for this system if the
/// manifest has one.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub version: String,
    pub notes: String,
    pub date: String,
    pub package: Option<Package>,
}

impl Found {
    pub fn page(&self) -> String {
        format!("{RELEASE_PAGE}{}", self.version)
    }
}

pub fn found_in(manifest: &Manifest, current: &str, platform: Option<&str>) -> Option<Found> {
    let newest = version_segments(&manifest.version);
    if newest.is_empty() || !is_newer(&newest, &version_segments(current)) {
        return None;
    }
    Some(Found {
        version: segments_text(&newest),
        notes: manifest.notes.clone(),
        date: manifest.pub_date.clone(),
        package: platform.and_then(|key| manifest.platforms.get(key).cloned()),
    })
}

// --- Where the request goes --------------------------------------------------------------

fn test_url() -> Option<String> {
    std::env::var(TEST_URL_ENV).ok().filter(|v| !v.is_empty())
}

fn manifest_url() -> String {
    test_url().unwrap_or_else(|| MANIFEST_URL.to_string())
}

fn public_key_text() -> String {
    match (test_url(), std::env::var(TEST_KEY_ENV)) {
        (Some(_), Ok(key)) if !key.is_empty() => key,
        _ => PUBLIC_KEY.to_string(),
    }
}

/// Packages come from the releases of SenseiIssei/Caruspex; a test run may
/// also name local files.
pub fn allowed_download(url: &str, testing: bool) -> bool {
    url.starts_with(DOWNLOAD_PREFIX)
        || (testing && (url.starts_with("https://") || url.starts_with("file://")))
}

/// file:///C:/x or file:///tmp/x to a path.
fn local_path(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    let bytes = rest.as_bytes();
    let rest = if bytes.len() > 2 && bytes[0] == b'/' && bytes[2] == b':' {
        &rest[1..]
    } else {
        rest
    };
    Some(PathBuf::from(rest))
}

fn agent() -> String {
    format!("Caruspex/{}", env!("CARGO_PKG_VERSION"))
}

/// Opens a download: the body as a reader and its length, if known.
fn open(
    url: &str,
    whole: Duration,
    testing: bool,
) -> Result<(Box<dyn Read + Send>, Option<u64>), String> {
    if testing && let Some(path) = local_path(url) {
        let file = File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let length = file.metadata().ok().map(|m| m.len());
        return Ok((Box::new(file), length));
    }
    // timeout_global: the time covers the whole request, from resolving the
    // name to the last byte of the body.
    let response = ureq::get(url)
        .header("User-Agent", &agent())
        .config()
        .timeout_global(Some(whole))
        .timeout_connect(Some(Duration::from_secs(15)))
        .build()
        .call()
        .map_err(|e| e.to_string())?;
    let length = response.body().content_length();
    Ok((Box::new(response.into_body().into_reader()), length))
}

/// Asks for the manifest. None when up to date, offline or anything else
/// goes wrong (offline: say nothing).
pub fn check(current: &str) -> Option<Found> {
    let (mut reader, _) = open(
        &manifest_url(),
        Duration::from_secs(8),
        test_url().is_some(),
    )
    .ok()?;
    let mut text = String::new();
    // A manifest is a few kilobytes; more is not a manifest.
    reader
        .by_ref()
        .take(1 << 20)
        .read_to_string(&mut text)
        .ok()?;
    let manifest = parse_manifest(&text).ok()?;
    let platform = platform_key(std::env::consts::OS, std::env::consts::ARCH);
    found_in(&manifest, current, platform.as_deref())
}

// --- Signatures --------------------------------------------------------------------------

fn decode_base64_text(text: &str) -> Result<String, String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(text.trim())
        .map_err(|e| e.to_string())?;
    String::from_utf8(raw).map_err(|e| e.to_string())
}

/// Checks a file against a signature of `tauri signer sign` and a public key
/// of `tauri signer generate`, both base64 as they write them. Only
/// prehashed signatures (all that tauri writes) are accepted.
pub fn verify_file(path: &Path, signature: &str, public_key: &str) -> Result<(), String> {
    let key = PublicKey::decode(&decode_base64_text(public_key)?).map_err(|e| e.to_string())?;
    let signature =
        Signature::decode(&decode_base64_text(signature)?).map_err(|e| e.to_string())?;
    let mut verifier = key.verify_stream(&signature).map_err(|e| e.to_string())?;
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        verifier.update(&buffer[..n]);
    }
    verifier.finalize().map_err(|e| e.to_string())
}

// --- How this copy updates itself --------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Method {
    /// Windows, installed with the Inno Setup installer.
    Installer,
    /// Linux, started as this AppImage file.
    AppImage(PathBuf),
    /// macOS, this .app bundle.
    Bundle(PathBuf),
    /// Only the download page: a .deb install, a portable copy, another system.
    PageOnly,
}

/// The .app bundle around .../Caruspex.app/Contents/MacOS/caruspex.
pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let is_bundle = macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && bundle.extension()? == "app";
    is_bundle.then(|| bundle.to_path_buf())
}

/// os as in std::env::consts::OS; appimage the APPIMAGE variable the
/// AppImage runtime sets.
pub fn detect_method(os: &str, exe: &Path, appimage: Option<&Path>) -> Method {
    match os {
        // The installer leaves its uninstaller next to the program; a copy
        // from the zip has none and would get a second installation.
        "windows" => match exe.parent() {
            Some(dir) if dir.join("unins000.exe").is_file() => Method::Installer,
            _ => Method::PageOnly,
        },
        "linux" => match appimage {
            Some(path) if path.is_file() => Method::AppImage(path.to_path_buf()),
            _ => Method::PageOnly,
        },
        "macos" => bundle_of(exe).map_or(Method::PageOnly, Method::Bundle),
        _ => Method::PageOnly,
    }
}

pub fn this_method() -> Method {
    let Ok(exe) = std::env::current_exe() else {
        return Method::PageOnly;
    };
    let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
    detect_method(std::env::consts::OS, &exe, appimage.as_deref())
}

/// Where to send someone whose copy cannot update itself.
pub fn fallback_url(found: &Found) -> String {
    if cfg!(target_os = "macos") {
        MAC_DMG.to_string()
    } else {
        found.page()
    }
}

// --- Installing --------------------------------------------------------------------------

/// What the interface does after install().
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The new version is in place or its installer runs: quit now.
    Quit,
    /// This copy cannot be replaced; the download is open in the browser.
    Page,
}

fn download_failed(error: impl std::fmt::Display) -> String {
    args(
        l(
            "Die Aktualisierung konnte nicht geladen werden: %1",
            "The update could not be downloaded: %1",
        ),
        &[&error],
    )
}

fn install_failed(error: impl std::fmt::Display) -> String {
    args(
        l(
            "Die Aktualisierung konnte nicht installiert werden: %1",
            "The update could not be installed: %1",
        ),
        &[&error],
    )
}

/// Loads a package into `target`, reporting (received, total) bytes; total
/// is 0 when the server does not say.
/// testing: a run with CARUSPEX_UPDATE_URL, which may also read file://.
pub fn download(
    url: &str,
    target: &Path,
    testing: bool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<(), String> {
    if !allowed_download(url, testing) {
        return Err(download_failed(url));
    }
    let (mut reader, length) =
        open(url, Duration::from_secs(30 * 60), testing).map_err(download_failed)?;
    let total = length.unwrap_or(0);
    let mut file = File::create(target).map_err(download_failed)?;
    let mut buffer = vec![0u8; 64 * 1024];
    let mut received = 0u64;
    progress(0, total);
    loop {
        let n = reader.read(&mut buffer).map_err(download_failed)?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).map_err(download_failed)?;
        received += n as u64;
        progress(received, total);
    }
    file.sync_all().map_err(download_failed)?;
    if total > 0 && received != total {
        return Err(download_failed(format!("{received} / {total} bytes")));
    }
    Ok(())
}

/// Loads and checks the package; a file whose signature does not match is
/// deleted at once.
fn fetch_verified(
    package: &Package,
    target: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<(), String> {
    if let Err(error) = download(&package.url, target, test_url().is_some(), progress) {
        let _ = std::fs::remove_file(target);
        return Err(error);
    }
    if verify_file(target, &package.signature, &public_key_text()).is_err() {
        let _ = std::fs::remove_file(target);
        return Err(l(
            "Die Signatur der Aktualisierung stimmt nicht. Sie wurde verworfen, nichts wurde installiert.",
            "The signature of the update does not match. It was discarded and nothing was installed.",
        )
        .to_string());
    }
    Ok(())
}

/// A fresh folder in the temp folder for the download.
fn work_dir() -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("caruspex-update");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(download_failed)?;
    Ok(dir)
}

fn file_name(url: &str) -> String {
    url.rsplit('/')
        .next()
        .filter(|name| !name.is_empty() && !name.contains(['\\', ':']))
        .unwrap_or("caruspex-update")
        .to_string()
}

/// Whether files can be created in this folder.
#[cfg_attr(windows, allow(dead_code))]
fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".caruspex-probe-{}", std::process::id()));
    let ok = File::create(&probe).is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Downloads, verifies and installs. On Quit the caller ends the app.
pub fn install(found: &Found, progress: &mut dyn FnMut(u64, u64)) -> Result<Outcome, String> {
    let method = this_method();
    let Some(package) = found
        .package
        .as_ref()
        .filter(|_| method != Method::PageOnly)
    else {
        return Err(l(
            "Für dieses System gibt es keine Aktualisierung zum Installieren.",
            "There is no update to install for this system.",
        )
        .to_string());
    };
    match method {
        Method::Installer => {
            let target = work_dir()?.join(file_name(&package.url));
            fetch_verified(package, &target, progress)?;
            std::process::Command::new(&target)
                .args(INNO_ARGUMENTS)
                .spawn()
                .map_err(install_failed)?;
            Ok(Outcome::Quit)
        }
        Method::AppImage(path) => install_appimage(&path, package, progress),
        Method::Bundle(bundle) => install_bundle(&bundle, package, progress),
        Method::PageOnly => Ok(Outcome::Page),
    }
}

#[cfg(target_os = "linux")]
fn install_appimage(
    path: &Path,
    package: &Package,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<Outcome, String> {
    use std::os::unix::fs::PermissionsExt;
    let Some(dir) = path.parent().filter(|dir| writable(dir)) else {
        return Ok(Outcome::Page);
    };
    // Next to the old file, so the rename below stays on one file system
    // and swaps the file in one step.
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let target = dir.join(format!(".{name}.update"));
    fetch_verified(package, &target, progress)?;
    let swap = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))
        .and_then(|_| std::fs::rename(&target, path));
    if let Err(error) = swap {
        let _ = std::fs::remove_file(&target);
        return Err(install_failed(error));
    }
    // The variables of the running AppImage point into its mount, which goes
    // away with this process; the new one sets its own.
    let mount = std::env::var("APPDIR").unwrap_or_default();
    let mut command = relaunch_command(path.as_os_str(), &[]);
    for (key, value) in std::env::vars_os() {
        let stale = ["APPDIR", "APPIMAGE", "ARGV0", "OWD"]
            .iter()
            .any(|k| key == *k)
            || (!mount.is_empty() && value.to_string_lossy().contains(&mount));
        if stale {
            command.env_remove(key);
        }
    }
    command.spawn().map_err(install_failed)?;
    Ok(Outcome::Quit)
}

#[cfg(not(target_os = "linux"))]
fn install_appimage(_: &Path, _: &Package, _: &mut dyn FnMut(u64, u64)) -> Result<Outcome, String> {
    Ok(Outcome::Page)
}

#[cfg(target_os = "macos")]
fn install_bundle(
    bundle: &Path,
    package: &Package,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<Outcome, String> {
    // In /Applications without admin rights the bundle cannot be replaced:
    // then the dmg opens instead.
    let Some(parent) = bundle
        .parent()
        .filter(|dir| writable(dir) && writable(bundle))
    else {
        return Ok(Outcome::Page);
    };
    let archive = work_dir()?.join("Caruspex.app.tar.gz");
    fetch_verified(package, &archive, progress)?;
    let pid = std::process::id();
    let staging = parent.join(format!(".caruspex-update-{pid}"));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(install_failed)?;
    let cleanup = |error: String| {
        let _ = std::fs::remove_dir_all(&staging);
        error
    };
    let unpacked = std::process::Command::new("/usr/bin/tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(&staging)
        .status()
        .map_err(|e| cleanup(install_failed(e)))?;
    if !unpacked.success() {
        return Err(cleanup(install_failed(unpacked)));
    }
    let new_bundle = std::fs::read_dir(&staging)
        .map_err(|e| cleanup(install_failed(e)))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .ok_or_else(|| cleanup(install_failed("Caruspex.app")))?;
    let old = parent.join(format!(".caruspex-old-{pid}.app"));
    std::fs::rename(bundle, &old).map_err(|e| cleanup(install_failed(e)))?;
    if let Err(error) = std::fs::rename(&new_bundle, bundle) {
        let _ = std::fs::rename(&old, bundle);
        return Err(cleanup(install_failed(error)));
    }
    let _ = std::fs::remove_dir_all(&old);
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_file(&archive);
    relaunch_command(
        std::ffi::OsStr::new("/usr/bin/open"),
        &[std::ffi::OsStr::new("-n"), bundle.as_os_str()],
    )
    .spawn()
    .map_err(install_failed)?;
    Ok(Outcome::Quit)
}

#[cfg(not(target_os = "macos"))]
fn install_bundle(_: &Path, _: &Package, _: &mut dyn FnMut(u64, u64)) -> Result<Outcome, String> {
    Ok(Outcome::Page)
}

/// A shell that waits until this process has ended and then starts the new
/// version, so the single-instance lock of the old one is gone by then.
#[cfg(unix)]
fn relaunch_command(
    program: &std::ffi::OsStr,
    arguments: &[&std::ffi::OsStr],
) -> std::process::Command {
    use std::process::Stdio;
    let mut command = std::process::Command::new("/bin/sh");
    command
        .arg("-c")
        .arg(r#"while kill -0 "$0" 2>/dev/null; do sleep 0.2; done; exec "$@""#)
        .arg(std::process::id().to_string())
        .arg(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/updater");

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(Path::new(DATA).join(name)).unwrap()
    }

    #[test]
    fn compares_like_qversionnumber() {
        assert_eq!(version_segments("v1.2.3-beta"), vec![1, 2, 3]);
        assert!(is_newer(
            &version_segments("v0.10.0"),
            &version_segments("0.9.0")
        ));
        assert!(!is_newer(
            &version_segments("0.9.0"),
            &version_segments("0.9.0")
        ));
        assert!(is_newer(
            &version_segments("0.9.0.1"),
            &version_segments("0.9.0")
        ));
        assert!(!is_newer(
            &version_segments("0.8.9"),
            &version_segments("0.9")
        ));
        assert!(is_newer(
            &version_segments("0.10.1"),
            &version_segments("0.10.0")
        ));
    }

    const MANIFEST: &str = r###"{
        "version": "0.10.1",
        "notes": "## Deutsch\n\nNeu:\n- Eins\n\n## English\n\nNew:\n- One\n",
        "pub_date": "2026-11-20T10:00:00Z",
        "platforms": {
            "windows-x86_64": {
                "url": "https://github.com/SenseiIssei/Caruspex/releases/download/v0.10.1/Caruspex-0.10.1-windows-x64-setup.exe",
                "signature": "c2ln"
            },
            "linux-x86_64": {
                "url": "https://github.com/SenseiIssei/Caruspex/releases/download/v0.10.1/Caruspex-0.10.1-linux-x86_64.AppImage",
                "signature": "c2ln"
            }
        }
    }"###;

    #[test]
    fn parses_the_manifest() {
        let m = parse_manifest(MANIFEST).unwrap();
        assert_eq!(m.version, "0.10.1");
        assert_eq!(m.pub_date, "2026-11-20T10:00:00Z");
        assert_eq!(m.platforms.len(), 2);
        assert!(m.platforms["windows-x86_64"].url.ends_with("-setup.exe"));
        // Notes and platforms may be missing, the version may not.
        assert!(parse_manifest(r#"{"version": "1.0.0"}"#).is_ok());
        assert!(parse_manifest(r#"{"version": "beta"}"#).is_err());
        assert!(parse_manifest(r#"{"notes": "x"}"#).is_err());
        assert!(parse_manifest("<html>").is_err());
    }

    #[test]
    fn finds_only_newer_releases_and_the_own_package() {
        let m = parse_manifest(MANIFEST).unwrap();
        let found = found_in(&m, "0.10.0", Some("windows-x86_64")).unwrap();
        assert_eq!(found.version, "0.10.1");
        assert!(
            found
                .package
                .unwrap()
                .url
                .ends_with("windows-x64-setup.exe")
        );
        assert_eq!(
            found_in(&m, "0.10.0", None).unwrap().page(),
            "https://github.com/SenseiIssei/Caruspex/releases/tag/v0.10.1"
        );
        // No macOS package in this manifest: newer, but nothing to install.
        assert!(
            found_in(&m, "0.10.0", Some("darwin-aarch64"))
                .unwrap()
                .package
                .is_none()
        );
        assert!(found_in(&m, "0.10.1", Some("windows-x86_64")).is_none());
        assert!(found_in(&m, "0.11.0", Some("windows-x86_64")).is_none());
    }

    #[test]
    fn picks_the_platform_key() {
        assert_eq!(
            platform_key("windows", "x86_64").as_deref(),
            Some("windows-x86_64")
        );
        assert_eq!(
            platform_key("linux", "x86_64").as_deref(),
            Some("linux-x86_64")
        );
        assert_eq!(
            platform_key("macos", "aarch64").as_deref(),
            Some("darwin-aarch64")
        );
        assert_eq!(
            platform_key("macos", "x86_64").as_deref(),
            Some("darwin-x86_64")
        );
        assert_eq!(platform_key("freebsd", "x86_64"), None);
        assert_eq!(platform_key("windows", "x86"), None);
    }

    #[test]
    fn splits_the_notes_by_language() {
        let notes = parse_manifest(MANIFEST).unwrap().notes;
        assert_eq!(notes_in(&notes, true), "Neu:\n- Eins");
        assert_eq!(notes_in(&notes, false), "New:\n- One");
        assert_eq!(notes_in("Just text\n", true), "Just text");
        // Only one part: that one for both.
        assert_eq!(notes_in("## English\nOnly English", true), "Only English");
        let changelog = include_str!("../../../changelog/0.10.0.md");
        assert!(notes_in(changelog, true).starts_with("Mehr Hilfe"));
        assert!(notes_in(changelog, false).starts_with("More help"));
    }

    #[test]
    fn accepts_the_signed_file() {
        let payload = Path::new(DATA).join("payload.bin");
        let signature = fixture("payload.bin.sig");
        let key = fixture("throwaway.pub");
        assert_eq!(verify_file(&payload, &signature, &key), Ok(()));
    }

    #[test]
    fn rejects_a_tampered_file_and_a_foreign_key() {
        let signature = fixture("payload.bin.sig");
        let key = fixture("throwaway.pub");
        let mut bytes = std::fs::read(Path::new(DATA).join("payload.bin")).unwrap();
        bytes[10] ^= 1;
        let dir = std::env::temp_dir().join(format!("caruspex-update-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let tampered = dir.join("payload.bin");
        std::fs::write(&tampered, &bytes).unwrap();
        assert!(verify_file(&tampered, &signature, &key).is_err());
        // The right file, but the release key did not sign it.
        let payload = Path::new(DATA).join("payload.bin");
        assert!(verify_file(&payload, &signature, PUBLIC_KEY).is_err());
        assert!(verify_file(&payload, "not base64!", &key).is_err());
        assert!(verify_file(&payload, &signature, "").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_release_key_decodes() {
        let text = decode_base64_text(PUBLIC_KEY).unwrap();
        assert!(text.contains("A2ADF3E331588ABC"));
        assert!(PublicKey::decode(&text).is_ok());
    }

    #[test]
    fn runs_inno_setup_silently_and_restarts() {
        for flag in [
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/CLOSEAPPLICATIONS",
            "/RESTARTAPPLICATIONS",
        ] {
            assert!(INNO_ARGUMENTS.contains(&flag), "{flag}");
        }
        // The parameter caruspex.iss reads to start the new version.
        assert!(INNO_ARGUMENTS.contains(&"/CARUSPEXRESTART=1"));
        let script = include_str!("../../../packaging/windows/caruspex.iss");
        assert!(script.contains("{param:CARUSPEXRESTART|0}"));
        // The normal start entry stays out of a silent run, or the app
        // would start twice.
        assert!(script.contains("postinstall skipifsilent"));
    }

    #[test]
    fn downloads_only_from_the_releases() {
        assert!(allowed_download(
            "https://github.com/SenseiIssei/Caruspex/releases/download/v0.10.1/x.exe",
            false
        ));
        assert!(!allowed_download("https://example.com/x.exe", false));
        assert!(!allowed_download(
            "https://github.com/someone/Caruspex/releases/download/v1/x.exe",
            false
        ));
        assert!(!allowed_download("file:///C:/x.exe", false));
        assert!(allowed_download("file:///C:/x.exe", true));
        assert_eq!(
            local_path("file:///C:/temp/x.exe"),
            Some(PathBuf::from("C:/temp/x.exe"))
        );
        assert_eq!(local_path("file:///tmp/x"), Some(PathBuf::from("/tmp/x")));
        assert_eq!(
            file_name("https://h/a/Caruspex-setup.exe"),
            "Caruspex-setup.exe"
        );
    }

    #[test]
    fn knows_how_this_copy_updates() {
        let dir = std::env::temp_dir().join(format!("caruspex-method-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("caruspex.exe");
        assert_eq!(detect_method("windows", &exe, None), Method::PageOnly);
        std::fs::write(dir.join("unins000.exe"), b"").unwrap();
        assert_eq!(detect_method("windows", &exe, None), Method::Installer);

        let image = dir.join("Caruspex.AppImage");
        assert_eq!(detect_method("linux", &exe, Some(&image)), Method::PageOnly);
        std::fs::write(&image, b"").unwrap();
        assert_eq!(
            detect_method("linux", &exe, Some(&image)),
            Method::AppImage(image.clone())
        );
        assert_eq!(detect_method("linux", &exe, None), Method::PageOnly);

        let mac = Path::new("/Applications/Caruspex.app/Contents/MacOS/caruspex");
        assert_eq!(
            detect_method("macos", mac, None),
            Method::Bundle(PathBuf::from("/Applications/Caruspex.app"))
        );
        assert_eq!(
            detect_method("macos", Path::new("/usr/local/bin/caruspex"), None),
            Method::PageOnly
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn downloads_and_verifies_a_local_package() {
        let dir = std::env::temp_dir().join(format!("caruspex-dl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("copy.bin");
        let source = Path::new(DATA).join("payload.bin");
        let path = source.display().to_string().replace('\\', "/");
        let url = format!(
            "file://{}{path}",
            if path.starts_with('/') { "" } else { "/" }
        );
        // Outside a test run a local file is refused.
        assert!(download(&url, &target, false, &mut |_, _| {}).is_err());
        let mut seen = Vec::new();
        download(&url, &target, true, &mut |received, total| {
            seen.push((received, total))
        })
        .unwrap();
        let length = std::fs::metadata(&source).unwrap().len();
        assert_eq!(seen.first(), Some(&(0, length)));
        assert_eq!(seen.last(), Some(&(length, length)));
        assert_eq!(
            verify_file(
                &target,
                &fixture("payload.bin.sig"),
                &fixture("throwaway.pub")
            ),
            Ok(())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
