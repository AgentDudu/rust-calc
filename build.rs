//! Give the exe a file icon.
//!
//! `Ui::icon()` paints the window and taskbar icon at runtime; Explorer reads the
//! icon from the PE's resources instead, so without this the exe shows Rust's
//! feather. Compiles `assets/icon.ico` with the Windows SDK resource compiler and
//! hands the result to the linker.
//!
//! `rc.exe` ships in the SDK `bin` directory and is usually not on `PATH`, so it
//! is looked for in `RC`, then `PATH`, then the SDK. If none is found the build
//! goes on without an icon rather than failing.

use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=RC");

    if std::env::var_os("CARGO_CFG_WINDOWS").is_none() {
        return;
    }
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    let res = Path::new(&out).join("rust-calc-res.res");

    let Some(rc) = find_rc() else {
        println!("cargo:warning=no rc.exe found - building without a file icon");
        return;
    };

    // Forward slashes: rc.exe takes them, and backslashes would need escaping.
    let ico = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
        .join("assets/icon.ico")
        .to_string_lossy()
        .replace('\\', "/");
    let rc_file = Path::new(&out).join("rust-calc-res.rc");
    // 1 = RT_ICON ordinal; the group entry is generated from it.
    std::fs::write(&rc_file, format!("1 ICON \"{ico}\"\n")).expect("write .rc");

    let status = std::process::Command::new(&rc)
        .args(["/nologo", "/fo"])
        .arg(&res)
        .arg(&rc_file)
        .status()
        .expect("run rc.exe");
    if !status.success() {
        println!("cargo:warning=rc.exe failed - building without a file icon");
        return;
    }
    println!("cargo:rustc-link-arg={}", res.to_string_lossy());
}

/// `RC`, then `PATH`, then the newest Windows SDK that has it.
fn find_rc() -> Option<PathBuf> {
    if let Some(from_env) = std::env::var_os("RC") {
        let p = PathBuf::from(from_env);
        if p.is_file() {
            return Some(p);
        }
    }
    // Only "did it spawn" matters here; rc's exit code for /? is not the question.
    if std::process::Command::new("rc.exe")
        .arg("/?")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
    {
        return Some(PathBuf::from("rc.exe"));
    }
    let kits = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut found: Vec<PathBuf> = std::fs::read_dir(kits)
        .ok()?
        .filter_map(std::result::Result::ok)
        .map(|e| e.path().join("x64/rc.exe"))
        .filter(|p| p.is_file())
        .collect();
    // Version-numbered directories sort correctly as strings at equal digit counts.
    found.sort();
    found.pop()
}
