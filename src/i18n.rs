// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Talkdedsec

//! English is the source language. The window's own text goes through Slint's
//! `@tr` and the catalog under `lang/`; this module covers what Rust renders:
//! status lines, preset names, the tray menu and file dialogs.

use std::fmt::{Display, Write};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    Tr,
}

impl Lang {
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Tr => "tr",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::En),
            "tr" => Some(Self::Tr),
            _ => None,
        }
    }

    /// The Windows display language, so a first run speaks the same language as
    /// the rest of the desktop. Anything other than Turkish gets English.
    pub fn system() -> Self {
        #[cfg(windows)]
        {
            use windows::Win32::Globalization::GetUserDefaultUILanguage;

            // The primary language id sits in the low ten bits of the LANGID.
            const LANG_TURKISH: u16 = 0x1f;
            if unsafe { GetUserDefaultUILanguage() } & 0x3ff == LANG_TURKISH {
                return Self::Tr;
            }
        }
        Self::En
    }
}

static TURKISH: AtomicBool = AtomicBool::new(false);

pub fn set(lang: Lang) {
    TURKISH.store(lang == Lang::Tr, Ordering::Relaxed);
}

pub fn current() -> Lang {
    if TURKISH.load(Ordering::Relaxed) {
        Lang::Tr
    } else {
        Lang::En
    }
}

/// `en` in the current language. A missing translation falls back to English.
pub fn t(en: &'static str) -> &'static str {
    text_in(current(), en)
}

pub fn text_in(lang: Lang, en: &'static str) -> &'static str {
    match lang {
        Lang::En => en,
        Lang::Tr => TURKISH_TEXT
            .iter()
            .find(|(source, _)| *source == en)
            .map_or(en, |(_, tr)| tr),
    }
}

/// Puts `args` into the `{}` slots of a template, in order. The slots move with
/// the language ("{}%" against "%{}"), which is why this is not `format!`.
pub fn fill(template: &str, args: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut args = args.iter();
    let mut rest = template;
    while let Some(at) = rest.find("{}") {
        out.push_str(&rest[..at]);
        if let Some(arg) = args.next() {
            let _ = write!(out, "{arg}");
        }
        rest = &rest[at + 2..];
    }
    out.push_str(rest);
    out
}

/// A slider value as the current language writes it: `1.25` or `1,25`.
pub fn number(value: f32, decimals: usize, signed: bool) -> String {
    number_in(current(), value, decimals, signed)
}

pub fn number_in(lang: Lang, value: f32, decimals: usize, signed: bool) -> String {
    let text = if signed {
        format!("{value:+.decimals$}")
    } else {
        format!("{value:.decimals$}")
    };
    match lang {
        Lang::En => text,
        Lang::Tr => text.replace('.', ","),
    }
}

const TURKISH_TEXT: &[(&str, &str)] = &[
    // engine and status line
    ("Ready.", "Hazır."),
    ("Waiting.", "Beklemede."),
    ("Applied.", "Uygulandı."),
    (
        "The display driver does not accept a gamma ramp.",
        "Ekran sürücüsü gama tablosunu kabul etmiyor.",
    ),
    (
        "Windows refused full strength — {}% applied.",
        "Windows tam gücü reddetti — %{} uygulandı.",
    ),
    (
        "Windows refused this setting.",
        "Windows bu ayarı reddetti.",
    ),
    ("Original image.", "Orijinal görüntü."),
    ("Back to defaults.", "Sıfırlandı."),
    ("Could not read the number.", "Sayı okunamadı."),
    ("Language changed.", "Dil değiştirildi."),
    // status detail
    (
        "Auto-apply is off — display untouched.",
        "Otomatik uygulama kapalı — ekran dokunulmadı.",
    ),
    ("Display untouched.", "Ekran dokunulmadı."),
    ("Brightness {}", "Parlaklık {}"),
    ("Contrast {}", "Kontrast {}"),
    ("Gamma {}", "Gama {}"),
    ("Temperature {}", "Sıcaklık {}"),
    ("Night vision {}", "Gece görüşü {}"),
    // profiles
    ("\"{}\" saved.", "\"{}\" kaydedildi."),
    ("\"{}\" loaded.", "\"{}\" yüklendi."),
    ("\"{}\" deleted.", "\"{}\" silindi."),
    ("A profile needs a name.", "Profil adı boş olamaz."),
    (
        "There are no profiles to export.",
        "Dışa aktarılacak profil yok.",
    ),
    ("Export profiles", "Profilleri dışa aktar"),
    ("Import profiles", "Profilleri içe aktar"),
    ("1 profile exported.", "1 profil dışa aktarıldı."),
    ("{} profiles exported.", "{} profil dışa aktarıldı."),
    ("1 profile imported.", "1 profil içe aktarıldı."),
    ("{} profiles imported.", "{} profil içe aktarıldı."),
    ("Export failed: {}", "Dışa aktarma hatası: {}"),
    ("Import failed: {}", "İçe aktarma hatası: {}"),
    // settings
    ("Will start with Windows.", "Windows açılışında başlayacak."),
    ("Start with Windows is off.", "Otomatik başlatma kapatıldı."),
    (
        "Could not write to the registry.",
        "Kayıt defterine yazılamadı.",
    ),
    ("Shortcut set to {}.", "Kısayol {} olarak ayarlandı."),
    (
        "{} is taken by another program.",
        "{} başka bir program tarafından kullanılıyor.",
    ),
    // tray menu
    ("Show window", "Pencereyi göster"),
    ("Turn the filter on or off", "Filtreyi aç / kapa"),
    ("Quit", "Çıkış"),
    // presets
    ("Clear", "Berrak"),
    ("a little crisper", "biraz daha net"),
    ("Night Vision", "Gece Görüşü"),
    ("detail in the dark", "karanlıkta detay"),
    ("Warm", "Sıcak"),
    ("easy on the eyes", "göz yormayan ton"),
    ("Cool", "Soğuk"),
    ("blue and sharp", "mavi ve sert"),
    ("Night Reading", "Gece Okuma"),
    ("dim and warm", "kısık ve sıcak"),
    ("Hard Contrast", "Sert Kontrast"),
    ("crushed shadows", "gölgeler kapanır"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    fn manifest(path: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
    }

    fn unescape(raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        }
        out
    }

    /// The string literal that opens `text`, without its quotes.
    fn literal(text: &str) -> Option<&str> {
        let body = text.strip_prefix('"')?;
        let mut escaped = false;
        for (i, c) in body.char_indices() {
            match c {
                '\\' if !escaped => escaped = true,
                '"' if !escaped => return Some(&body[..i]),
                _ => escaped = false,
            }
        }
        None
    }

    /// Every literal passed straight to `t(...)` in the Rust sources.
    fn rust_messages() -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for entry in std::fs::read_dir(manifest("src")).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|n| n == "i18n.rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for (at, _) in source.match_indices("t(") {
                let before = source[..at].chars().next_back();
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                if let Some(text) = literal(source[at + 2..].trim_start()) {
                    found.insert(unescape(text));
                }
            }
        }
        found
    }

    /// Every `@tr("...")` in the Slint sources.
    fn slint_messages() -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        for entry in std::fs::read_dir(manifest("ui")).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "slint") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for (at, _) in source.match_indices("@tr(") {
                let text = literal(source[at + 4..].trim_start()).expect("@tr without a literal");
                found.insert(unescape(text));
            }
        }
        found
    }

    /// msgid → msgstr from the bundled Turkish catalog, header excluded.
    fn catalog() -> Vec<(String, String)> {
        let po = std::fs::read_to_string(manifest("lang/tr/LC_MESSAGES/talkdedsec-visual.po"))
            .unwrap()
            .replace("\r\n", "\n");
        let mut pairs = Vec::new();
        let mut id = None;
        for line in po.lines() {
            if let Some(rest) = line.strip_prefix("msgid ") {
                id = Some(unescape(literal(rest).unwrap()));
            } else if let Some(rest) = line.strip_prefix("msgstr ") {
                let id = id.take().expect("msgstr without msgid");
                if !id.is_empty() {
                    pairs.push((id, unescape(literal(rest).unwrap())));
                }
            }
        }
        pairs
    }

    fn slots(text: &str) -> usize {
        text.matches("{}").count()
    }

    #[test]
    fn codes_round_trip() {
        for lang in [Lang::En, Lang::Tr] {
            assert_eq!(Lang::from_code(lang.code()), Some(lang));
        }
        assert_eq!(Lang::from_code("de"), None);
        assert_eq!(Lang::from_code(""), None);
    }

    #[test]
    fn english_is_the_source_text() {
        for (en, _) in TURKISH_TEXT {
            assert_eq!(text_in(Lang::En, en), *en);
        }
    }

    #[test]
    fn turkish_looks_up_and_falls_back() {
        assert_eq!(text_in(Lang::Tr, "Ready."), "Hazır.");
        assert_eq!(text_in(Lang::Tr, "not in the table"), "not in the table");
    }

    #[test]
    fn numbers_follow_the_language() {
        assert_eq!(number_in(Lang::En, 0.1, 2, true), "+0.10");
        assert_eq!(number_in(Lang::Tr, 0.1, 2, true), "+0,10");
        assert_eq!(number_in(Lang::En, 1.0, 2, false), "1.00");
        assert_eq!(number_in(Lang::Tr, -0.26, 2, true), "-0,26");
    }

    #[test]
    fn fill_places_arguments_in_order() {
        assert_eq!(fill("{}% applied", &[&70]), "70% applied");
        assert_eq!(fill("%{} uygulandı", &[&70]), "%70 uygulandı");
        assert_eq!(fill("{} and {}", &[&"a", &"b"]), "a and b");
        assert_eq!(fill("no slots", &[&1]), "no slots");
    }

    #[test]
    fn turkish_table_is_consistent() {
        let mut seen = BTreeSet::new();
        for (en, tr) in TURKISH_TEXT {
            assert!(seen.insert(*en), "listed twice: {en:?}");
            assert!(!tr.trim().is_empty(), "empty translation: {en:?}");
            assert_eq!(slots(en), slots(tr), "placeholders differ: {en:?}");
        }
    }

    #[test]
    fn every_rust_message_is_translated_and_every_translation_used() {
        let mut used = rust_messages();
        assert!(
            used.len() > 20,
            "the source scan found too little: {used:?}"
        );
        for preset in crate::presets::ALL {
            used.insert(preset.name.to_string());
            used.insert(preset.hint.to_string());
        }

        let table: BTreeSet<String> = TURKISH_TEXT.iter().map(|(en, _)| en.to_string()).collect();
        let missing: Vec<_> = used.difference(&table).collect();
        let unused: Vec<_> = table.difference(&used).collect();
        assert!(missing.is_empty(), "no Turkish for: {missing:?}");
        assert!(unused.is_empty(), "translated but never shown: {unused:?}");
    }

    #[test]
    fn slint_catalog_matches_the_ui() {
        let used = slint_messages();
        assert!(
            used.len() > 40,
            "the source scan found too little: {used:?}"
        );

        let pairs = catalog();
        let mut ids = BTreeSet::new();
        for (id, tr) in &pairs {
            assert!(ids.insert(id.clone()), "listed twice: {id:?}");
            assert!(!tr.trim().is_empty(), "empty translation: {id:?}");
            assert_eq!(slots(id), slots(tr), "placeholders differ: {id:?}");
        }

        let missing: Vec<_> = used.difference(&ids).collect();
        let unused: Vec<_> = ids.difference(&used).collect();
        assert!(missing.is_empty(), "no Turkish for: {missing:?}");
        assert!(unused.is_empty(), "translated but never shown: {unused:?}");
    }
}
