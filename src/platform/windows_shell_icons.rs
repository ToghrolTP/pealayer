//! Deterministic, multi-resolution ICO generation from our existing Phosphor
//! font. Used at build time only (and by focused resource regression tests).

use ab_glyph::{Font, FontRef, PxScale, point};

pub const SHELL_ICON_SIZES: [u32; 7] = [16, 20, 24, 32, 40, 48, 64];

pub fn shell_icon_rgba(symbol: &str, size: u32) -> Vec<u8> {
    let font = FontRef::try_from_slice(egui_phosphor::Variant::Regular.font_bytes())
        .expect("bundled Phosphor font");
    let id = font.glyph_id(symbol.chars().next().expect("shell icon glyph"));
    assert_ne!(id.0, 0, "shell icon is missing from Phosphor");
    let scale = PxScale::from(size as f32 * 0.9);
    let outline = font
        .outline_glyph(id.with_scale(scale))
        .expect("shell icon outline");
    let bounds = outline.px_bounds();
    let position = point(
        (size as f32 - bounds.width()) * 0.5 - bounds.min.x,
        (size as f32 - bounds.height()) * 0.5 - bounds.min.y,
    );
    let outline = font
        .outline_glyph(id.with_scale_and_position(scale, position))
        .expect("positioned shell icon outline");
    let bounds = outline.px_bounds();
    let mut pixels = vec![0; (size * size * 4) as usize];
    outline.draw(|x, y, coverage| {
        let x = bounds.min.x.floor() as i32 + x as i32;
        let y = bounds.min.y.floor() as i32 + y as i32;
        if x >= 0 && y >= 0 && x < size as i32 && y < size as i32 {
            let offset = ((y as u32 * size + x as u32) * 4) as usize;
            // Muted blue-gray remains visible on both light and dark shell
            // surfaces without baking in the user's current theme or accent.
            pixels[offset..offset + 4].copy_from_slice(&[
                126,
                132,
                142,
                (coverage * 255.0).round() as u8,
            ]);
        }
    });
    pixels
}

pub fn shell_icon_ico(symbol: &str) -> Vec<u8> {
    let mut frames = Vec::new();
    for size in SHELL_ICON_SIZES {
        let rgba = shell_icon_rgba(symbol, size);
        let mask_stride = size.div_ceil(32) * 4;
        let mut bitmap = Vec::new();
        for value in [40, size, size * 2] {
            bitmap.extend(value.to_le_bytes());
        }
        bitmap.extend(1u16.to_le_bytes());
        bitmap.extend(32u16.to_le_bytes());
        for value in [0u32, size * size * 4, 0, 0, 0, 0] {
            bitmap.extend(value.to_le_bytes());
        }
        // ICO DIBs use bottom-up BGRA pixels followed by a DWORD-aligned AND
        // mask. Include it even with alpha for older shell extraction paths.
        for y in (0..size).rev() {
            for x in 0..size {
                let offset = ((y * size + x) * 4) as usize;
                bitmap.extend([
                    rgba[offset + 2],
                    rgba[offset + 1],
                    rgba[offset],
                    rgba[offset + 3],
                ]);
            }
        }
        for y in (0..size).rev() {
            let mut mask = vec![0u8; mask_stride as usize];
            for x in 0..size {
                if rgba[((y * size + x) * 4 + 3) as usize] == 0 {
                    mask[(x / 8) as usize] |= 0x80 >> (x % 8);
                }
            }
            bitmap.extend(mask);
        }
        frames.push((size, bitmap));
    }
    let mut ico = vec![0, 0, 1, 0];
    ico.extend((frames.len() as u16).to_le_bytes());
    let mut offset = 6 + frames.len() as u32 * 16;
    for (size, bitmap) in &frames {
        ico.extend([*size as u8, *size as u8, 0, 0]);
        ico.extend(1u16.to_le_bytes());
        ico.extend(32u16.to_le_bytes());
        ico.extend((bitmap.len() as u32).to_le_bytes());
        ico.extend(offset.to_le_bytes());
        offset += bitmap.len() as u32;
    }
    for (_, bitmap) in frames {
        ico.extend(bitmap);
    }
    ico
}
