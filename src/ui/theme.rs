//! The design system: every color and spacing value the UI uses comes from here.

use eframe::egui::{self, Color32, Visuals};

use crate::constants::layout::MIN_TARGET_SIZE;
use crate::models::{ChangeKind, DiffLineKind};
use crate::services::settings::ThemeChoice;

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub accent: Color32,
    pub muted_text: Color32,
    pub danger: Color32,
    pub added: Color32,
    pub removed: Color32,
    pub renamed: Color32,
    pub modified: Color32,
    pub conflicted: Color32,
    pub added_background: Color32,
    pub removed_background: Color32,
    pub hunk_background: Color32,
    pub toolbar_background: Color32,
    pub error_background: Color32,
}

const LIGHT: Palette = Palette {
    accent: Color32::from_rgb(0x09, 0x69, 0xda),
    muted_text: Color32::from_rgb(0x59, 0x63, 0x6e),
    danger: Color32::from_rgb(0xcf, 0x22, 0x2e),
    added: Color32::from_rgb(0x1a, 0x7f, 0x37),
    removed: Color32::from_rgb(0xcf, 0x22, 0x2e),
    renamed: Color32::from_rgb(0x82, 0x50, 0xdf),
    modified: Color32::from_rgb(0x9a, 0x67, 0x00),
    conflicted: Color32::from_rgb(0x9a, 0x67, 0x00),
    added_background: Color32::from_rgb(0xda, 0xfb, 0xe1),
    removed_background: Color32::from_rgb(0xff, 0xeb, 0xe9),
    hunk_background: Color32::from_rgb(0xdd, 0xf4, 0xff),
    toolbar_background: Color32::from_rgb(0x24, 0x29, 0x2f),
    error_background: Color32::from_rgb(0xff, 0xeb, 0xe9),
};

const DARK: Palette = Palette {
    accent: Color32::from_rgb(0x4c, 0x9b, 0xff),
    muted_text: Color32::from_rgb(0x9d, 0xa7, 0xb3),
    danger: Color32::from_rgb(0xff, 0x7b, 0x72),
    added: Color32::from_rgb(0x3f, 0xb9, 0x50),
    removed: Color32::from_rgb(0xf8, 0x51, 0x49),
    renamed: Color32::from_rgb(0xbc, 0x8c, 0xff),
    modified: Color32::from_rgb(0xd2, 0x99, 0x22),
    conflicted: Color32::from_rgb(0xd2, 0x99, 0x22),
    added_background: Color32::from_rgb(0x12, 0x3d, 0x22),
    removed_background: Color32::from_rgb(0x4a, 0x1a, 0x1d),
    hunk_background: Color32::from_rgb(0x12, 0x2d, 0x4a),
    toolbar_background: Color32::from_rgb(0x16, 0x1b, 0x22),
    error_background: Color32::from_rgb(0x4a, 0x1a, 0x1d),
};

pub fn palette(visuals: &Visuals) -> Palette {
    if visuals.dark_mode { DARK } else { LIGHT }
}

/// Text color on the always-dark toolbar, in both themes.
pub const TOOLBAR_TEXT: Color32 = Color32::from_rgb(0xf0, 0xf6, 0xfc);

/// Name of egui's bundled monospace font, which also covers the arrows and symbols the UI uses.
const SYMBOL_FALLBACK_FONT: &str = "Hack";

/// Keeps egui's default typeface and adds the bundled monospace font as a fallback, so symbols like
/// arrows render instead of failing on fonts that lack them.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional)
        && !family.iter().any(|name| name == SYMBOL_FALLBACK_FONT)
    {
        family.push(SYMBOL_FALLBACK_FONT.to_owned());
    }
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context, theme: ThemeChoice) {
    ctx.set_theme(match theme {
        ThemeChoice::System => egui::ThemePreference::System,
        ThemeChoice::Light => egui::ThemePreference::Light,
        ThemeChoice::Dark => egui::ThemePreference::Dark,
    });
    // Both the light and dark styles, so switching theme keeps the sizes.
    ctx.all_styles_mut(|style| {
        style.spacing.interact_size.y = MIN_TARGET_SIZE;
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    });
}

pub fn change_color(palette: &Palette, kind: ChangeKind) -> Color32 {
    match kind {
        ChangeKind::Added | ChangeKind::Untracked | ChangeKind::Copied => palette.added,
        ChangeKind::Deleted => palette.removed,
        ChangeKind::Renamed => palette.renamed,
        ChangeKind::Conflicted => palette.conflicted,
        ChangeKind::Modified | ChangeKind::TypeChanged => palette.modified,
    }
}

/// Short letter shown in the file list, plus a readable name for screen readers.
pub fn change_label(kind: ChangeKind) -> (&'static str, &'static str) {
    match kind {
        ChangeKind::Added => ("A", "Added"),
        ChangeKind::Untracked => ("+", "New"),
        ChangeKind::Modified => ("M", "Modified"),
        ChangeKind::Deleted => ("D", "Deleted"),
        ChangeKind::Renamed => ("R", "Renamed"),
        ChangeKind::Copied => ("C", "Copied"),
        ChangeKind::TypeChanged => ("T", "Type changed"),
        ChangeKind::Conflicted => ("!", "Conflicted"),
    }
}

pub fn diff_background(palette: &Palette, kind: DiffLineKind) -> Color32 {
    match kind {
        DiffLineKind::Added => palette.added_background,
        DiffLineKind::Removed => palette.removed_background,
        DiffLineKind::Hunk => palette.hunk_background,
        _ => Color32::TRANSPARENT,
    }
}
