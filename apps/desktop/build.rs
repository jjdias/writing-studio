//! On Windows, put the icon and the version into the executable, so the
//! file shows the nib in Explorer and its version in Properties. The window
//! takes its own icon at run time (`src/main.rs`); this is the file's.

fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("icons/icon.ico")
            .set("ProductName", "Writing Studio")
            .set("FileDescription", "Writing Studio")
            .set("LegalCopyright", "Writing Studio contributors, MIT OR Apache-2.0");
        // A computer without the Windows resource compiler still builds the
        // program, with the plain icon Windows gives every executable.
        if let Err(err) = resource.compile() {
            println!("cargo:warning=no icon in the executable: {err}");
        }
    }
}
