//! Embeds `assets/moonhunt.ico` as the executable's icon resource, so Explorer,
//! the taskbar and Alt-Tab show the MoonHunt logo instead of the generic exe icon.
//!
//! ponytail: the icon needs `rc.exe`; probe PATH first, then the SDK roots on
//! this machine. Upgrade path: the `winresource` build-dep if the project ever
//! has to build where the Windows SDK lives somewhere unknown.
use std::path::{Path, PathBuf};
use std::process::Command;

const SDK_ROOTS: [&str; 2] = [
    r"C:\Program Files (x86)\Windows Kits\10\bin",
    r"D:\Windows Kits\10\bin",
];

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    println!("cargo:rerun-if-changed=assets/moonhunt.ico");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let ico = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/moonhunt.ico");
    let rc = out.join("moonhunt.rc");
    std::fs::write(&rc, format!("1 ICON \"{}\"\n", ico.display())).expect("write .rc");

    let res = out.join("moonhunt.res");
    let Some(compiler) = find_rc() else {
        println!("cargo:warning=no rc.exe found — the exe will have no icon resource");
        return;
    };
    let status = Command::new(&compiler)
        .arg("/fo")
        .arg(&res)
        .arg(&rc)
        .status()
        .expect("run the resource compiler");
    if !status.success() {
        println!(
            "cargo:warning={} failed — the exe will have no icon resource",
            compiler.display()
        );
        return;
    }
    println!("cargo:rustc-link-arg-bins={}", res.display());
}

/// `rc.exe` from PATH, else the newest Windows SDK found under `SDK_ROOTS`.
fn find_rc() -> Option<PathBuf> {
    for exe in ["rc.exe", "llvm-rc.exe"] {
        if let Ok(out) = Command::new(exe).arg("/?").output() {
            if out.status.code().is_some() {
                return Some(PathBuf::from(exe));
            }
        }
    }
    for root in SDK_ROOTS {
        let mut versions: Vec<PathBuf> = std::fs::read_dir(root)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path().join("x64").join("rc.exe"))
            .filter(|p| p.is_file())
            .collect();
        versions.sort();
        if let Some(rc) = versions.pop() {
            return Some(rc);
        }
    }
    None
}
