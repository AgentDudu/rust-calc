//! Fluent-ish palettes, the theme toggle, and the window icon.
//!
//! Windows' own calculator follows the system theme, so that is the default here too — egui
//! already tracks it (`ThemePreference::System`). This module only replaces egui's greys with
//! Windows 11 colours and adds a three-way button for people who want to force one.

use eframe::egui::{self, Color32, Stroke, Theme, Visuals};

// Icon palette — shared by `icon()` below and `assets/icon.ico` (tools/make_icon.py).
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0x00, 0x5F, 0xB8);
const ACCENT_DARK: Color32 = Color32::from_rgb(0x4C, 0xC2, 0xFF);
pub const DISPLAY: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x1B);
pub const BARS: Color32 = Color32::from_rgb(0xCF, 0xE4, 0xFA);

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
                Color32::TRANSPARENT
            } else {
                glyph(x, y)
            };
            let at = ((y * SIZE + x) * 4) as usize;
            rgba[at..at + 4].copy_from_slice(&pixel.to_array());
        }
    }
    egui::IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    }
}

fn inside_rounded(x: u32, y: u32, size: u32, radius: u32) -> bool {
    let (left, right) = (x < radius, x > size - radius - 1);
    let (top, bottom) = (y < radius, y > size - radius - 1);
    // Only the corner this pixel actually sits in gets a circle test — testing all four
    // would reject every pixel in a corner square, which reads as a chamfer, not a radius.
    let corner = match (left, right, top, bottom) {
        (true, _, true, _) => (radius, radius),
        (_, true, true, _) => (size - radius - 1, radius),
        (true, _, _, true) => (radius, size - radius - 1),
        (_, true, _, true) => (size - radius - 1, size - radius - 1),
        _ => return true,
    };
    let (dx, dy) = (x as i32 - corner.0 as i32, y as i32 - corner.1 as i32);
    dx * dx + dy * dy <= (radius * radius) as i32
}

/// A display bar over an equals sign: readable at 16 px, which is all a taskbar icon has to be.
fn glyph(x: u32, y: u32) -> Color32 {
    if (7..=10).contains(&y) && (7..=24).contains(&x) {
        DISPLAY
    } else if (10..=21).contains(&x) && ((15..=17).contains(&y) || (21..=23).contains(&y)) {
        BARS
    } else {
        ACCENT_LIGHT
    }
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
        assert_eq!(
            alpha(4, 4),
            255,
            "a rounded corner keeps the pixel beside it"
        );
        assert_eq!(alpha(16, 16), 255, "centre should be solid");
        assert_eq!(alpha(16, 8), 255);
        let colour = |x: u32, y: u32| {
            let at = ((y * icon.width + x) * 4) as usize;
            Color32::from_rgba_premultiplied(
                icon.rgba[at],
                icon.rgba[at + 1],
                icon.rgba[at + 2],
                icon.rgba[at + 3],
            )
        };
        assert_eq!(colour(10, 8), DISPLAY, "the display band");
        assert_eq!(colour(15, 16), BARS, "the equals bars");
        assert_eq!(colour(4, 4), ACCENT_LIGHT, "the tile");
    }

    /// The exe's file icon is a separate asset from the drawn one, so nothing stops it drifting.
    /// This is the drift check: the sizes Explorer asks for, and the colours `icon()` paints.
    #[test]
    fn the_file_icon_matches_the_drawn_one() {
        let ico = std::fs::read("assets/icon.ico")
            .expect("missing assets/icon.ico - run tools/make_icon.py");
        assert_eq!(&ico[0..4], &[0, 0, 1, 0], "assets/icon.ico is not an ICO");
        let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;

        let mut sizes = Vec::new();
        let mut thirty_two = None;
        for i in 0..count {
            let entry = &ico[6 + i * 16..][..16];
            // 0 in the width byte is how an ICO spells 256.
            let size = if entry[0] == 0 {
                256
            } else {
                entry[0] as usize
            };
            let len = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
            let at = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
            if size == 32 {
                thirty_two = Some((at, len));
            }
            sizes.push(size);
        }
        sizes.sort_unstable();
        assert_eq!(
            sizes,
            [16, 24, 32, 48, 256],
            "taskbar, small, window, large, Explorer"
        );

        let (at, len) = thirty_two.expect("a 32 px entry");
        assert_eq!(
            &ico[at..at + 4],
            &[40, 0, 0, 0],
            "expected a 32 bpp BMP entry"
        );
        let pixels = &ico[at + 40..at + len]; // bottom-up BGRA
        assert_eq!(pixels.len() % 4, 0, "whole BGRA pixels");
        let has = |colour: Color32| {
            let c = colour.to_array();
            (0..pixels.len())
                .step_by(4)
                .any(|i| [pixels[i + 2], pixels[i + 1], pixels[i]] == [c[0], c[1], c[2]])
        };
        assert!(has(ACCENT_LIGHT), "tile colour missing from the .ico");
        assert!(has(DISPLAY), "display band colour missing from the .ico");
        assert!(has(BARS), "equals bars colour missing from the .ico");
    }
}
