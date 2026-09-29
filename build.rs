fn main() {
    // Rebuild if the icon changes so the embedded resource stays in sync.
    println!("cargo:rerun-if-changed=assets/icon.ico");

    // Embed the application icon into the Windows executable so RaydioSurfer
    // shows a proper icon in Explorer, the taskbar, and the Alt-Tab switcher.
    // This is a no-op on non-Windows platforms.
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(err) = res.compile() {
            // Don't hard-fail the build if the resource compiler is missing;
            // the runtime window icon is still applied via set_window_icon.
            println!("cargo:warning=failed to embed Windows icon: {err}");
        }
    }
}
