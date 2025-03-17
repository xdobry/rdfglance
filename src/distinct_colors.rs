use eframe::egui::Color32;

pub fn next_distinct_color(taken_colors: usize, saturation: f32, lightness: f32) -> Color32 {
    let hue = (taken_colors as f32 / PHI) * 360.0 % 360.0; // Golden ratio spacing
    let (r, g, b) = hsl_to_rgb(hue, saturation, lightness);
    Color32::from_rgb(r, g, b)
}

/// Convert HSL to RGB (values 0-255)
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

// Golden Ratio constant for best color distribution
const PHI: f32 = 1.61803398875; 