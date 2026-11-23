//! The game's build script: the larger stack on MSVC, and on Windows the
//! launcher's icon and version details in `ffe-rs.exe`. A missing resource
//! compiler only loses the icon; it never fails the build.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // The simulation/render loop's debug stack frame exceeds Windows' 1 MiB
        // executable default. Keep the larger reserve on this binary only.
        println!("cargo:rustc-link-arg-bin=ffe-rs=/STACK:8388608");
    }
    println!("cargo:rerun-if-changed=../ffe-launcher/assets/icon.ico");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("../ffe-launcher/assets/icon.ico")
            .set(
                "FileDescription",
                "Frontier: First Encounters (Elite 3), Rust port",
            )
            .set("ProductName", "ffe-rs");
        if let Err(error) = resource.compile() {
            println!("cargo:warning=game icon not embedded: {error}");
        }
    }
}
