//! On Windows, embeds the DH icon and product name into the executable, so Explorer, the
//! taskbar and the file's Properties dialog show them. Elsewhere this is a no-op.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_resources();
    }
}

#[cfg(windows)]
fn embed_windows_resources() {
    let mut resource = winresource::WindowsResource::new();
    resource.set_icon("assets/icon.ico").set("ProductName", "DHMIX").set("FileDescription", "DHMIX mixer for streaming");
    if let Err(e) = resource.compile() {
        println!("cargo:warning=could not embed the Windows icon: {e}");
    }
}

#[cfg(not(windows))]
fn embed_windows_resources() {}
