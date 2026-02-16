use egui::{FontData, FontDefinitions, FontFamily};
use font_kit::{
    family_name::FamilyName,
    properties::{Properties, Weight},
    source::SystemSource,
};
use std::sync::{Arc, LazyLock};

pub static SANS_SERIF: LazyLock<FontFamily> =
    LazyLock::new(|| FontFamily::Name("sans-serif".into()));
pub static SERIF: LazyLock<FontFamily> = LazyLock::new(|| FontFamily::Name("serif".into()));

pub static SANS_SERIF_BOLD: LazyLock<FontFamily> =
    LazyLock::new(|| FontFamily::Name("sans-serif-bold".into()));
pub static SERIF_BOLD: LazyLock<FontFamily> =
    LazyLock::new(|| FontFamily::Name("serif-bold".into()));

fn find_system_font(names: &[FamilyName], bold: bool) -> Option<Vec<u8>> {
    let source = SystemSource::new();
    let mut properties = Properties::new();
    if bold {
        properties.weight(Weight::BOLD);
    }

    if let Ok(handle) = source.select_best_match(names, &properties)
        && let Ok(font) = handle.load()
    {
        return Some(font.copy_font_data()?.to_vec());
    }
    None
}

pub fn load(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let sans_fonts: &[FamilyName] = &[
        FamilyName::Title("Microsoft YaHei".to_string()), // Windows
        FamilyName::Title("SimHei".to_string()),          // Windows
        FamilyName::Title("Source Han Sans CN".to_string()), // Open Source
        FamilyName::Title("Noto Sans CJK SC".to_string()), // Google/Adobe
        FamilyName::Title("WenQuanYi Micro Hei".to_string()), // Linux fallback
        FamilyName::Title("PingFang SC".to_string()),     // macOS
        FamilyName::Title("Heiti SC".to_string()),        // macOS
        FamilyName::SansSerif, // fallback to system sans-serif if no preferred found
        FamilyName::Serif,     // fallback to serif if no sans found
    ];

    let serif_fonts: &[FamilyName] = &[
        FamilyName::Title("SimSun".to_string()),   // Windows
        FamilyName::Title("NSimSun".to_string()),  // Windows
        FamilyName::Title("FangSong".to_string()), // Windows
        FamilyName::Title("KaiTi".to_string()),    // Windows
        FamilyName::Title("Source Han Serif CN".to_string()), // Open Source
        FamilyName::Title("Noto Serif CJK SC".to_string()), // Open Source
        FamilyName::Title("STSong".to_string()),   // macOS
        FamilyName::Title("Songti SC".to_string()), // macOS
        FamilyName::Title("STFangsong".to_string()), // macOS
        FamilyName::Title("STKaiti".to_string()),  // macOS
        FamilyName::Serif,     // fallback to system serif if no preferred found
        FamilyName::SansSerif, // fallback to sans-serif if no serif found
    ];

    let font_configs = [
        (sans_fonts, "sans-serif", SANS_SERIF.clone(), false),
        (sans_fonts, "sans-serif-bold", SANS_SERIF_BOLD.clone(), true),
        (serif_fonts, "serif", SERIF.clone(), false),
        (serif_fonts, "serif-bold", SERIF_BOLD.clone(), true),
    ];

    for (names, id, family, bold) in font_configs {
        if let Some(data) = find_system_font(names, bold) {
            fonts
                .font_data
                .insert(id.to_owned(), Arc::new(FontData::from_owned(data)));

            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, id.to_owned());
        }
    }
    ctx.set_fonts(fonts);
}
