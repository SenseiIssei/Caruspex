//! Builds the fake PassThru driver (fake-j2534/) for the J2534 tests. Each
//! test file that loads it is its own process, so the global state of the
//! DLL is never shared between two tests running at once.

use std::path::PathBuf;
use std::process::Command;

/// Builds the fake PassThru driver and returns the path of its DLL.
pub fn fake_driver() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fake-j2534")
        .join("Cargo.toml");
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fake-j2534");
    // The output (linker notes included) only matters when the build fails.
    let output = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&target)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_TARGET")
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "fake-j2534 builds:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    target.join("debug").join("fake_j2534.dll")
}
