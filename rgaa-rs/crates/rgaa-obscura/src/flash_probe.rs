//! Flash analysis of animated GIFs (issue #273, RGAA 13.7 — GIF part).
//!
//! WCAG general flash threshold: no more than three flashes in any one-second period,
//! where a flash is a pair of opposing changes in relative luminance on a sufficiently
//! large area (21 824 px, the figure the issue fixes). Per consecutive pair of frames we
//! count the pixels whose relative luminance moved by at least [`LUMINANCE_STEP`], sign
//! the transition by whether those pixels got brighter or darker, keep the transitions
//! covering at least [`MIN_FLASH_AREA_PX`], and count each **reversal of sign** between
//! successive kept transitions as one flash. The answer is the highest count inside any
//! one-second window of the animation's timeline.
//!
//! Known limits, kept explicit:
//! APNG, video, CSS and script animation are **not** analysed here; a clean report
//! never proves conformity (the mechanism stays `partial`).

use std::io::Cursor;

/// Minimum relative-luminance change (0..1) for a pixel to count as changed.
pub const LUMINANCE_STEP: f64 = 0.1;
/// Minimum changed area for a transition to count toward a flash.
pub const MIN_FLASH_AREA_PX: u32 = 21_824;
/// More flashes than this in any one second exceeds the threshold.
pub const MAX_FLASHES_PER_SECOND: u32 = 3;

/// Result of analysing one GIF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashReport {
    pub frames: usize,
    /// Highest flash count inside any one-second window.
    pub max_flashes_per_second: u32,
    /// Largest changed area between two consecutive frames, in pixels.
    pub max_changed_area_px: u32,
}

impl FlashReport {
    #[must_use]
    pub fn exceeds_threshold(&self) -> bool {
        self.max_flashes_per_second > MAX_FLASHES_PER_SECOND
    }
}

fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    let lin = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// Analyses an animated GIF.
///
/// # Errors
/// Fails when the bytes are not a decodable GIF.
pub fn analyze_gif(bytes: &[u8]) -> Result<FlashReport, String> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options
        .read_info(Cursor::new(bytes))
        .map_err(|e| format!("not a decodable GIF: {e}"))?;
    let (w, h) = (usize::from(decoder.width()), usize::from(decoder.height()));
    let mut canvas = vec![[0u8; 4]; w * h];
    let mut previous: Option<Vec<f64>> = None;
    let mut time_ms: u64 = 0;
    let mut frames = 0usize;
    let mut max_area = 0u32;
    // (timestamp in ms, sign) of each kept transition.
    let mut kept: Vec<(u64, i8)> = Vec::new();

    while let Some(frame) = decoder
        .read_next_frame()
        .map_err(|e| format!("GIF frame error: {e}"))?
    {
        frames += 1;
        let (fl, ft) = (usize::from(frame.left), usize::from(frame.top));
        let (fw, fh) = (usize::from(frame.width), usize::from(frame.height));
        let restore = (frame.dispose == gif::DisposalMethod::Previous).then(|| canvas.clone());
        for y in 0..fh {
            for x in 0..fw {
                let src = &frame.buffer[(y * fw + x) * 4..(y * fw + x) * 4 + 4];
                if src[3] != 0 && ft + y < h && fl + x < w {
                    canvas[(ft + y) * w + fl + x] = [src[0], src[1], src[2], 255];
                }
            }
        }
        let lum: Vec<f64> = canvas
            .iter()
            .map(|p| relative_luminance(p[0], p[1], p[2]))
            .collect();
        if let Some(prev) = &previous {
            let (mut up, mut down) = (0u32, 0u32);
            for (a, b) in prev.iter().zip(&lum) {
                let d = b - a;
                if d >= LUMINANCE_STEP {
                    up += 1;
                } else if d <= -LUMINANCE_STEP {
                    down += 1;
                }
            }
            let area = up + down;
            max_area = max_area.max(area);
            if area >= MIN_FLASH_AREA_PX {
                kept.push((time_ms, if up >= down { 1 } else { -1 }));
            }
        }
        previous = Some(lum);
        time_ms += u64::from(frame.delay.max(1)) * 10;
        if let Some(before) = restore {
            canvas = before;
        } else if frame.dispose == gif::DisposalMethod::Background {
            for y in 0..fh {
                for x in 0..fw {
                    if ft + y < h && fl + x < w {
                        canvas[(ft + y) * w + fl + x] = [0, 0, 0, 0];
                    }
                }
            }
        }
    }

    // One flash per sign reversal between successive kept transitions.
    let reversals: Vec<u64> = kept
        .windows(2)
        .filter(|pair| pair[0].1 != pair[1].1)
        .map(|pair| pair[1].0)
        .collect();
    let max_flashes_per_second = reversals
        .iter()
        .enumerate()
        .map(|(i, start)| {
            reversals[i..]
                .iter()
                .take_while(|t| **t < start + 1000)
                .count() as u32
        })
        .max()
        .unwrap_or(0);

    Ok(FlashReport {
        frames,
        max_flashes_per_second,
        max_changed_area_px: max_area,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a `side`×`side` GIF alternating black/white every `delay_cs` centiseconds.
    fn alternating(side: u16, frames: usize, delay_cs: u16) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let palette = [0, 0, 0, 255, 255, 255];
            let mut enc = gif::Encoder::new(&mut out, side, side, &palette).unwrap();
            for i in 0..frames {
                let mut frame = gif::Frame {
                    width: side,
                    height: side,
                    delay: delay_cs,
                    buffer: std::borrow::Cow::Owned(vec![
                        (i % 2) as u8;
                        usize::from(side) * usize::from(side)
                    ]),
                    ..gif::Frame::default()
                };
                frame.dispose = gif::DisposalMethod::Keep;
                enc.write_frame(&frame).unwrap();
            }
        }
        out
    }

    #[test]
    fn restore_previous_disposal_reverts_the_canvas() {
        // Black base, then white frames disposed "restore previous" interleaved with
        // fully transparent frames: the visible image alternates white/black.
        let side = 200u16;
        let n = usize::from(side) * usize::from(side);
        let mut out = Vec::new();
        {
            let palette = [0, 0, 0, 255, 255, 255];
            let mut enc = gif::Encoder::new(&mut out, side, side, &palette).unwrap();
            let frame = |index: u8, dispose, transparent| gif::Frame {
                width: side,
                height: side,
                delay: 10,
                dispose,
                transparent,
                buffer: std::borrow::Cow::Owned(vec![index; n]),
                ..gif::Frame::default()
            };
            enc.write_frame(&frame(0, gif::DisposalMethod::Keep, None))
                .unwrap();
            for _ in 0..15 {
                enc.write_frame(&frame(1, gif::DisposalMethod::Previous, None))
                    .unwrap();
                enc.write_frame(&frame(0, gif::DisposalMethod::Keep, Some(0)))
                    .unwrap();
            }
        }
        let report = analyze_gif(&out).unwrap();
        assert!(report.max_flashes_per_second > 3, "{report:?}");
    }

    #[test]
    fn fast_alternation_on_a_large_area_exceeds_the_threshold() {
        // 200×200 = 40 000 px, 10 fps alternation: far above 3 flashes/s.
        let report = analyze_gif(&alternating(200, 30, 10)).unwrap();
        assert!(report.exceeds_threshold(), "{report:?}");
        assert!(report.max_changed_area_px >= MIN_FLASH_AREA_PX);
    }

    #[test]
    fn slow_alternation_does_not_exceed() {
        // One change every 1.2 s: at most one reversal per second.
        let report = analyze_gif(&alternating(200, 10, 120)).unwrap();
        assert!(!report.exceeds_threshold(), "{report:?}");
    }

    #[test]
    fn fast_alternation_on_a_small_area_is_not_a_flash() {
        // 50×50 = 2 500 px, well under the area threshold.
        let report = analyze_gif(&alternating(50, 30, 10)).unwrap();
        assert!(!report.exceeds_threshold(), "{report:?}");
    }

    #[test]
    fn a_static_gif_has_no_flash() {
        let report = analyze_gif(&alternating(200, 1, 10)).unwrap();
        assert_eq!(report.frames, 1);
        assert_eq!(report.max_flashes_per_second, 0);
    }

    #[test]
    fn non_gif_bytes_are_an_error() {
        assert!(analyze_gif(b"not a gif").is_err());
    }
}
