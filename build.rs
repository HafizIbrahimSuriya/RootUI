// Embeds the RootUI icon (rootui.ico) and version info into the Windows exe.
fn main() {
    println!("cargo:rerun-if-changed=rootui.ico");
    println!("cargo:rerun-if-changed=build.rs");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("rootui.ico");
        res.set("ProductName", "RootUI");
        res.set("CompanyName", "gitsquare");
        res.set("FileDescription", "RootUI");
        res.set("OriginalFilename", "RootUI.exe");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the exe icon: {e}");
        }
    }
}
