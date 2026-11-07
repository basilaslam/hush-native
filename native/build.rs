fn main() {
    #[cfg(target_os = "windows")]
    {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .set("ProductName", "Hush")
            .set("FileDescription", "Hush native peer-to-peer voice")
            .compile()
            .expect("Could not embed the Windows app icon");
    }
}
