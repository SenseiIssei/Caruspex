//! Builds the fake PassThru driver (fake-j2534/) for the J2534 tests, and
//! for the bridge tests a 32-bit build of it with the 32-bit helper program.
//! Each test file that loads it is its own process, so the global state of
//! the DLL is never shared between two tests running at once.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// The target of the 32-bit bridge.
pub const TARGET_32: &str = "i686-pc-windows-msvc";

fn cargo_build(manifest: &Path, target_dir: &Path, extra: &[&str]) {
    // The output (linker notes included) only matters when the build fails.
    let output = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(manifest)
        .arg("--target-dir")
        .arg(target_dir)
        .args(extra)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_TARGET")
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "{} builds:\n{}",
        manifest.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fake_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fake-j2534")
        .join("Cargo.toml")
}

/// Builds the fake PassThru driver and returns the path of its DLL.
pub fn fake_driver() -> PathBuf {
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fake-j2534");
    cargo_build(&fake_manifest(), &target, &[]);
    target.join("debug").join("fake_j2534.dll")
}

/// Whether the standard library for 32-bit Windows is installed
/// (`rustup target add i686-pc-windows-msvc`).
pub fn target_32_installed() -> bool {
    let rustc = Path::new(env!("CARGO")).with_file_name("rustc");
    let Ok(out) = Command::new(&rustc).args(["--print", "sysroot"]).output() else {
        return false;
    };
    let sysroot = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Path::new(&sysroot)
        .join("lib")
        .join("rustlib")
        .join(TARGET_32)
        .join("lib")
        .is_dir()
}

/// The fake driver built as a 32-bit DLL.
pub fn fake_driver_32() -> PathBuf {
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fake-j2534-32");
    cargo_build(&fake_manifest(), &target, &["--target", TARGET_32]);
    target.join(TARGET_32).join("debug").join("fake_j2534.dll")
}

/// The 32-bit helper program caruspex-j2534-bridge.
pub fn bridge_32() -> PathBuf {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("bridge-32");
    cargo_build(
        &workspace.join("Cargo.toml"),
        &target,
        &[
            "-p",
            "caruspex-j2534",
            "--bin",
            "caruspex-j2534-bridge",
            "--target",
            TARGET_32,
        ],
    );
    target
        .join(TARGET_32)
        .join("debug")
        .join("caruspex-j2534-bridge.exe")
}
