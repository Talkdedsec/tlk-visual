fn main() {
    // English is the source text in ui/*.slint; Turkish ships inside the binary.
    // The catalog carries no msgctxt, so strings are not keyed by component name.
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("lang")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/main.slint", config).expect("slint build failed");
    println!("cargo:rerun-if-changed=lang");

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
