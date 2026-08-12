fn main() {
    slint_build::compile("ui/main.slint").expect("slint build failed");

    #[cfg(windows)]
    if std::path::Path::new("assets/icon.ico").exists() {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Talkdedsec Visual");
        res.set("FileDescription", "Talkdedsec Visual");
        res.set("LegalCopyright", "Copyright (C) 2026 Talkdedsec");
        res.compile().expect("resource build failed");
    }
}
