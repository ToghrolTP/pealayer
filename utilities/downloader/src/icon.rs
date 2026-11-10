// Code-native utility glyph: a download arrow and tray, distinct from the player logo.
pub fn rgba(size: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let (u, v) = (
                (x as f32 + 0.5) / size as f32,
                (y as f32 + 0.5) / size as f32,
            );
            let corner = ((u - 0.5).abs() - 0.29)
                .max(0.0)
                .hypot(((v - 0.5).abs() - 0.29).max(0.0));
            let alpha = ((0.19 - corner) * size as f32).clamp(0.0, 1.0);
            let arrow = (0.445..=0.555).contains(&u) && (0.20..=0.53).contains(&v)
                || (0.48..=0.70).contains(&v) && (u - 0.5).abs() <= 0.70 - v
                || (0.28..=0.72).contains(&u) && (0.77..=0.82).contains(&v)
                || (0.27..=0.32).contains(&u) && (0.68..=0.82).contains(&v)
                || (0.68..=0.73).contains(&u) && (0.68..=0.82).contains(&v);
            let color = if arrow {
                [243, 249, 255]
            } else {
                [38, (101.0 + 33.0 * v) as u8, (190.0 + 35.0 * u) as u8]
            };
            bytes.extend_from_slice(&[color[0], color[1], color[2], (alpha * 255.0) as u8]);
        }
    }
    bytes
}
