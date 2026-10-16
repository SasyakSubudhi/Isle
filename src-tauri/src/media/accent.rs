//! One accent colour from album artwork, for theming the pill while music plays.

/// The most prominent colourful hue in `pixels`, lifted to a lightness that
/// reads on the dark pill. Grey or near-black artwork gives `None`, so the
/// pill stays monochrome.
pub fn accent(pixels: impl Iterator<Item = [u8; 3]>) -> Option<String> {
    // 4 bits per channel; weight each pixel by how colourful and mid-tone it is.
    let mut buckets = vec![(0f64, [0f64; 3]); 4096];
    let mut total = 0f64;
    for [r, g, b] in pixels {
        let (_, s, l) = hsl([r, g, b]);
        let weight = s * (1.0 - (2.0 * l - 1.0).abs());
        total += 1.0;
        if weight < 0.12 {
            continue;
        }
        let index = (usize::from(r >> 4) << 8) | (usize::from(g >> 4) << 4) | usize::from(b >> 4);
        let bucket = &mut buckets[index];
        bucket.0 += weight;
        bucket.1[0] += f64::from(r) * weight;
        bucket.1[1] += f64::from(g) * weight;
        bucket.1[2] += f64::from(b) * weight;
    }
    let (weight, sum) = buckets.into_iter().max_by(|a, b| a.0.total_cmp(&b.0))?;
    // Ignore a stray coloured speck on otherwise grey art.
    if total == 0.0 || weight / total < 0.004 {
        return None;
    }
    let rgb = sum.map(|c| (c / weight).round().clamp(0.0, 255.0) as u8);
    let (h, s, _) = hsl(rgb);
    let [r, g, b] = from_hsl(h, s.clamp(0.45, 0.85), 0.66);
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

fn hsl([r, g, b]: [u8; 3]) -> (f64, f64, f64) {
    let (r, g, b) = (
        f64::from(r) / 255.0,
        f64::from(g) / 255.0,
        f64::from(b) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d == 0.0 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } * 60.0;
    (h, s, l)
}
fn from_hsl(h: f64, s: f64, l: f64) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r, g, b].map(|v| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn picks_the_dominant_colour_over_grey() {
        let mut pixels = vec![[30, 30, 30]; 900];
        pixels.extend(vec![[200, 40, 40]; 80]);
        pixels.extend(vec![[40, 40, 200]; 20]);
        let hex = accent(pixels.into_iter()).unwrap();
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap();
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap();
        assert!(r > 200 && b < 140, "{hex}");
    }
    #[test]
    fn grey_artwork_keeps_the_pill_monochrome() {
        assert_eq!(accent(vec![[128, 128, 128]; 500].into_iter()), None);
        assert_eq!(accent(std::iter::empty()), None);
    }
}
