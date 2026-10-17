//! Fluent-ish palettes, the theme toggle, and the window icon.
//!
//! Windows' own calculator follows the system theme, so that is the default here too — egui
//! already tracks it (`ThemePreference::System`). This module only replaces egui's greys with
//! Windows 11 colours and adds a three-way button for people who want to force one.

use eframe::egui::{self, Color32, Stroke, Theme, Visuals};

const ACCENT_LIGHT: Color32 = Color32::from_rgb(0x00, 0x5F, 0xB8);
const ACCENT_DARK: Color32 = Color32::from_rgb(0x4C, 0xC2, 0xFF);

/// Installs both palettes once. egui then swaps them on its own when the OS theme changes,
/// so nothing here has to run per frame.
pub fn install(ctx: &egui::Context) {
    ctx.set_visuals_of(Theme::Dark, dark());
    ctx.set_visuals_of(Theme::Light, light());
}

/// System → Light → Dark → System. The three states Windows users argue about.
pub fn cycle(preference: egui::ThemePreference) -> egui::ThemePreference {
    use egui::ThemePreference::{Dark, Light, System};
    match preference {
        System => Light,
        Light => Dark,
        Dark => System,
    }
}

pub fn label(preference: egui::ThemePreference) -> &'static str {
    match preference {
        egui::ThemePreference::System => "Theme: system",
        egui::ThemePreference::Light => "Theme: light",
        egui::ThemePreference::Dark => "Theme: dark",
    }
}

fn dark() -> Visuals {
    let mut visuals = Visuals::dark();
    paint(&mut visuals, 0x20, 0x27, 0x2D);
    let white = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        widget.fg_stroke.color = white;
    }
    // `disabled` is deliberately left alone: it is what makes a greyed key read as dead.
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x2D, 0x2D, 0x2D);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x45, 0x45, 0x45));
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x35, 0x35, 0x35);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x4C, 0x4C, 0x4C));
    visuals.widgets.active.bg_fill = Color32::from_rgb(0x28, 0x28, 0x28);
    // Black on the pale accent, as Windows does.
    visuals.widgets.open.bg_fill = ACCENT_DARK;
    visuals.widgets.open.fg_stroke.color = Color32::from_rgb(0x00, 0x00, 0x00);
    visuals.selection.bg_fill = ACCENT_DARK;
    visuals
}

fn light() -> Visuals {
    let mut visuals = Visuals::light();
    paint(&mut visuals, 0xF3, 0xFB, 0xF0);

    let ink = Color32::from_rgb(0x1A, 0x1A, 0x1A);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        widget.fg_stroke.color = ink;
    }
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0xFD, 0xFD, 0xFD);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xE0, 0xE0, 0xE0));
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0xF5, 0xF5, 0xF5);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0xD0, 0xD0, 0xD0));
    visuals.widgets.active.bg_fill = Color32::from_rgb(0xEA, 0xEA, 0xEA);
    visuals.widgets.open.bg_fill = ACCENT_LIGHT;
    visuals.widgets.open.fg_stroke.color = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    visuals.selection.bg_fill = ACCENT_LIGHT;
    visuals
}

/// `base` is the window, `layer` the panels sitting on it, `faint` the recesses.
/// `extreme_bg_color` must match `layer`, not `base`: eframe clears the window with it, and every
/// edge of this app is a panel, so anything else shows as a seam while resizing.
fn paint(visuals: &mut Visuals, base: u8, layer: u8, faint: u8) {
    visuals.window_fill = Color32::from_rgb(base, base, base);
    visuals.panel_fill = Color32::from_rgb(layer, layer, layer);
    visuals.extreme_bg_color = Color32::from_rgb(layer, layer, layer);
    visuals.faint_bg_color = Color32::from_rgb(faint, faint, faint);
}

/// 32x32 window icon, drawn here rather than shipped as a `.ico` blob: no image crate, no binary
/// in the repo, and it survives a `cargo package`. No anti-aliasing — see §7 for the tradeoff.
/// The *executable file's* icon in Explorer still needs a real `.ico` resource.
pub fn icon() -> egui::IconData {
    const SIZE: u32 = 32;
    const RADIUS: u32 = 6;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let pixel = if !inside_rounded(x, y, SIZE, RADIUS) {
                [0, 0, 0, 0]
            } else if is_glyph(x, y) {
                [0xFF, 0xFF, 0xFF, 0xFF]
            } else {
                [0x00, 0x78, 0xD4, 0xFF]
            };
            let at = ((y * SIZE + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&pixel);
        }
    }
    egui::IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    }
}

fn inside_rounded(x: u32, y: u32, size: u32, radius: u32) -> bool {
    let corners = [
        (radius, radius),
        (size - radius - 1, radius),
        (radius, size - radius - 1),
        (size - radius - 1, size - radius - 1),
    ];
    // Outside a corner square only if it is also outside that corner's circle.
    for (cx, cy) in corners {
        let in_corner_box =
            (x < radius || x > size - radius - 1) && (y < radius || y > size - radius - 1);
        if !in_corner_box {
            continue;
        }
        let (dx, dy) = (x as i32 - cx as i32, y as i32 - cy as i32);
        if dx * dx + dy * dy > (radius * radius) as i32 {
            return false;
        }
    }
    true
}

/// A display bar over an equals sign: readable at 16 px, which is all a taskbar icon has to be.
fn is_glyph(x: u32, y: u32) -> bool {
    (7..=10).contains(&y) && (7..=24).contains(&x)
        || (15..=17).contains(&y) && (10..=21).contains(&x)
        || (21..=23).contains(&y) && (10..=21).contains(&x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_palettes_are_actually_different() {
        let (dark, light) = (dark(), light());
        assert_ne!(dark.window_fill, light.window_fill);
        assert_ne!(
            dark.widgets.inactive.bg_fill,
            light.widgets.inactive.bg_fill
        );
        assert_ne!(dark.widgets.open.bg_fill, light.widgets.open.bg_fill);
    }

    #[test]
    fn the_clear_colour_matches_the_panels_so_no_seam_shows() {
        for visuals in [dark(), light()] {
            assert_eq!(visuals.extreme_bg_color, visuals.panel_fill);
            assert_ne!(
                visuals.window_fill, visuals.panel_fill,
                "layers need to read as layers"
            );
        }
    }

    #[test]
    fn the_toggle_visits_every_theme_and_comes_back() {
        use egui::ThemePreference::{Dark, Light, System};
        assert_eq!(cycle(System), Light);
        assert_eq!(cycle(Light), Dark);
        assert_eq!(cycle(Dark), System);
        let labels = [label(System), label(Light), label(Dark)];
        assert!(labels.iter().all(|text| !text.is_empty()));
        assert_eq!(labels[0], labels[1].replace("light", "system"));
    }

    #[test]
    fn installing_a_palette_survives_a_theme_switch() {
        let ctx = egui::Context::default();
        install(&ctx);
        ctx.set_theme(Theme::Dark);
        let dark_fill = ctx.global_style().visuals.window_fill;
        ctx.set_theme(Theme::Light);
        let light_fill = ctx.global_style().visuals.window_fill;
        assert_ne!(dark_fill, light_fill, "install() did not reach both themes");
        assert_eq!(light_fill, light().window_fill);
    }

    #[test]
    fn the_icon_is_a_rounded_tile_with_transparent_corners() {
        let icon = icon();
        assert_eq!(icon.width % 4, 0);
        assert_eq!(icon.height % 4, 0);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
        let alpha = |x: u32, y: u32| icon.rgba[((y * icon.width + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0, "corner should be cut");
        assert_eq!(alpha(16, 16), 255, "centre should be solid");
        assert_eq!(alpha(16, 8), 255);
        let glyph = |x: u32, y: u32| icon.rgba[((y * icon.width + x) * 4) as usize];
        assert_eq!(glyph(10, 8), 0xFF, "the display bar should be white");
        assert_ne!(glyph(16, 30), 0xFF, "the tile body should not be white");
    }
}
