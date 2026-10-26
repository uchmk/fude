//! The app's icon: one square of 原稿用紙, ruled in vermilion, with a brush's
//! stroke in it, on the dark tile the uchmk apps share. Drawn here at any
//! size so no image file is carried.

type Rgb = [u8; 3];

/// The tile's colour and its rim (the theme's `bg` and `line`).
const TILE: Rgb = [0x16, 0x18, 0x1d];
const RIM: Rgb = [0x2c, 0x30, 0x39];
/// The square's rule (the mock's `--shu`) and the ink, the colour of paper.
const SHU: Rgb = [0xd0, 0x50, 0x3f];
const INK: Rgb = [0xf2, 0xed, 0xe2];

/// The square, in the tile's 32-unit box: its centre, half its side and its
/// rule's width.
const SQUARE: ([f32; 2], f32, f32) = ([16.0, 16.0], 9.5, 1.8);

/// The stroke, pressed in at the top left and lifted to a point at the
/// bottom right: where it passes and how wide it is there (a radius).
const STROKE: [([f32; 2], f32); 4] = [([11.6, 10.8], 3.3), ([15.0, 13.6], 3.0), ([18.4, 17.6], 2.1), ([21.0, 21.6], 0.7)];

/// The icon as RGBA, `size` × `size`.
pub fn pixels(size: u32) -> Vec<u8> {
    let n = size as usize;
    let unit = size as f32 / 32.0;
    // Thicker at small sizes so the rule survives 16 px.
    let boost = if size <= 16 {
        1.4
    } else if size <= 32 {
        1.15
    } else {
        1.0
    };
    let (centre, half, rule) = SQUARE;
    let (centre, half, rule) = (centre.map(|v| v * unit), half * unit, rule * unit * boost);
    let stroke: Vec<([f32; 2], f32)> = STROKE.iter().map(|(p, r)| (p.map(|v| v * unit), r * unit)).collect();
    let radius = 7.3 * unit;
    let mut out = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let p = [x as f32 + 0.5, y as f32 + 0.5];
            let tile = rounded(p, size as f32, 0.0, radius);
            if tile <= 0.0 {
                continue;
            }
            let mut rgb = TILE.map(f32::from);
            let inside = rounded(p, size as f32, unit, radius - unit);
            blend(&mut rgb, RIM, 1.0 - inside);
            // How far out of the square's edge (inside is below 0).
            let edge = (p[0] - centre[0]).abs().max((p[1] - centre[1]).abs()) - half;
            blend(&mut rgb, SHU, cover(rule / 2.0 - edge.abs()));
            blend(&mut rgb, INK, cover(-tapered(p, &stroke)));
            let i = (y * n + x) * 4;
            out[i..i + 3].copy_from_slice(&rgb.map(|v| v.round() as u8));
            out[i + 3] = (tile * 255.0).round() as u8;
        }
    }
    out
}

/// How much of a pixel a shape covers, from how far inside it the pixel's
/// centre is.
fn cover(inside: f32) -> f32 {
    (inside + 0.5).clamp(0.0, 1.0)
}

fn blend(rgb: &mut [f32; 3], color: Rgb, cover: f32) {
    for k in 0..3 {
        rgb[k] += (f32::from(color[k]) - rgb[k]) * cover;
    }
}

/// How far `p` is out of the stroke (below 0 inside it): the nearest of its
/// pieces, each a line whose width goes from the one end's to the other's.
fn tapered(p: [f32; 2], stroke: &[([f32; 2], f32)]) -> f32 {
    stroke
        .windows(2)
        .map(|s| {
            let ((a, ra), (b, rb)) = (s[0], s[1]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            let (qx, qy) = (a[0] + dx * t, a[1] + dy * t);
            ((p[0] - qx).powi(2) + (p[1] - qy).powi(2)).sqrt() - (ra + (rb - ra) * t)
        })
        .fold(f32::INFINITY, f32::min)
}

/// How much of the pixel at `p` is inside the square of `side`, inset by
/// `inset` and rounded by `radius`.
fn rounded(p: [f32; 2], side: f32, inset: f32, radius: f32) -> f32 {
    let (lo, hi) = (inset + radius, side - inset - radius);
    let (cx, cy) = (p[0].clamp(lo, hi), p[1].clamp(lo, hi));
    cover(radius - ((p[0] - cx).powi(2) + (p[1] - cy).powi(2)).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(p: &[u8], size: usize, x: usize, y: usize) -> [u8; 4] {
        let i = (y * size + x) * 4;
        [p[i], p[i + 1], p[i + 2], p[i + 3]]
    }

    #[test]
    fn it_is_square_rgba_with_clear_corners() {
        let p = pixels(32);
        assert_eq!(p.len(), 32 * 32 * 4);
        assert_eq!(p[3], 0);
        assert_eq!(p[(16 * 32 + 16) * 4 + 3], 255);
    }

    /// At 256 px: the square's rule is vermilion, the stroke's head ink, and
    /// the paper in the square's corner the tile.
    #[test]
    fn the_square_and_the_stroke_are_there() {
        let s = 256;
        let p = pixels(s as u32);
        let unit = s as f32 / 32.0;
        let px = |v: f32| (v * unit) as usize;
        assert_eq!(at(&p, s, px(16.0), px(6.5))[..3], SHU);
        assert_eq!(at(&p, s, px(11.6), px(10.8))[..3], INK);
        assert_eq!(at(&p, s, px(23.0), px(9.0))[..3], TILE);
        // Nothing of the stroke past its lifted point.
        assert_eq!(at(&p, s, px(22.4), px(23.4))[..3], TILE);
    }
}
