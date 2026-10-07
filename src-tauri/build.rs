use std::error::Error;

use tauri_build::{Attributes, WindowsAttributes};

fn main() -> Result<(), Box<dyn Error>> {
    // Release builds ask for administrator rights at launch: reading a volume's file
    // table needs them. Debug builds (`tauri dev`, `cargo test`) keep running as the
    // user and index by walking folders instead.
    let mut windows = WindowsAttributes::new();
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        windows = windows.app_manifest(include_str!("findex.manifest"));
    }
    println!("cargo:rerun-if-changed=findex.manifest");
    tauri_build::try_build(Attributes::new().windows_attributes(windows))?;

    // tauri's generated context leaves `bundle.copyright` out of the runtime config,
    // so hand it to the app as a compile-time env var; tauri.conf.json stays its
    // single source, shared with the installer.
    let conf: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("tauri.conf.json")?)?;
    let copyright = conf["bundle"]["copyright"].as_str().unwrap_or_default();
    println!("cargo:rustc-env=FINDEX_COPYRIGHT={copyright}");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    Ok(())
}
