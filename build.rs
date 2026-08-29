//! Embeds the Windows icon and version metadata into the executable.
//!
//! Without this the binary shows the generic Windows icon in Explorer and the
//! taskbar, and its Properties panel is blank.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/sxarty.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon/sxarty.ico");
        resource.set("FileDescription", "Sxarty");
        resource.set("ProductName", "Sxarty");
        resource.set("OriginalFilename", "sxarty.exe");
        resource.set(
            "LegalCopyright",
            "MIT licensed. Copyright (c) 2026 Beqa Abuladze.",
        );

        if let Err(error) = resource.compile() {
            // A missing resource compiler should not stop a developer building
            // and running the app; only the metadata is lost.
            println!("cargo:warning=could not embed Windows resources: {error}");
        }
    }
}
