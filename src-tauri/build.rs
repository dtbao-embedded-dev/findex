use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tauri_build::build();
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
