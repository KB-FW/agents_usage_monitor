//! Tray icon drawn as a small gauge of the highest usage.

use tauri::image::Image;

const SIZE: u32 = 64;
const BASE: [f64; 3] = [0x2B as f64, 0x2B as f64, 0x2B as f64];
const TRACK: [f64; 3] = [0x55 as f64, 0x55 as f64, 0x55 as f64];
const OK: [f64; 3] = [0xEE as f64, 0xEE as f64, 0xEE as f64];
const ACCENT: [f64; 3] = [0xF0 as f64, 0x5C as f64, 0x1A as f64];
const CRIT: [f64; 3] = [0xFF as f64, 0x45 as f64, 0x3A as f64];

/// `pct` = highest used percentage (None = no data yet).
pub fn gauge(pct: Option<f64>, warn: f64, crit: f64) -> Image<'static> {
    let fill = match pct {
        Some(p) if p >= crit => CRIT,
        Some(p) if p >= warn => ACCENT,
        _ => OK,
    };
    let sweep = pct.map(|p| p.clamp(0.0, 100.0) / 100.0 * std::f64::consts::TAU);
    let c = SIZE as f64 / 2.0;
    let mut buf = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (mut r, mut g, mut b, mut a) = (0.0, 0.0, 0.0, 0.0);
            for sy in 0..4 {
                for sx in 0..4 {
                    let dx = x as f64 + (sx as f64 + 0.5) / 4.0 - c;
                    let dy = y as f64 + (sy as f64 + 0.5) / 4.0 - c;
                    let d = (dx * dx + dy * dy).sqrt();
                    let col = if d > 31.0 {
                        None
                    } else if (18.5..=27.5).contains(&d) {
                        let ang = (dx.atan2(-dy) + std::f64::consts::TAU) % std::f64::consts::TAU;
                        Some(if sweep.map(|s| ang <= s).unwrap_or(false) {
                            fill
                        } else {
                            TRACK
                        })
                    } else if d <= 7.0 {
                        Some(ACCENT)
                    } else {
                        Some(BASE)
                    };
                    if let Some(cl) = col {
                        r += cl[0];
                        g += cl[1];
                        b += cl[2];
                        a += 1.0;
                    }
                }
            }
            let i = ((y * SIZE + x) * 4) as usize;
            if a > 0.0 {
                buf[i] = (r / a).round() as u8;
                buf[i + 1] = (g / a).round() as u8;
                buf[i + 2] = (b / a).round() as u8;
                buf[i + 3] = (a / 16.0 * 255.0).round() as u8;
            }
        }
    }
    Image::new_owned(buf, SIZE, SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_a_round_icon() {
        let img = gauge(Some(50.0), 80.0, 95.0);
        let px = |x: u32, y: u32| {
            let i = ((y * SIZE + x) * 4) as usize;
            img.rgba()[i..i + 4].to_vec()
        };
        assert_eq!(px(0, 0)[3], 0, "corners are transparent");
        assert_eq!(px(32, 32), vec![0xF0, 0x5C, 0x1A, 255], "accent centre");
        assert_eq!(px(55, 32)[0], 0xEE, "right side of the ring is within 50 %");
        assert_eq!(px(9, 32)[0], 0x55, "left side is track");
    }
}
