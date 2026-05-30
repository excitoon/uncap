use anyhow::{Context, Result};
use image::DynamicImage;

/// Solve a slide/jigsaw captcha.
/// Given the full background image with a notch and the puzzle piece,
/// returns the X offset (in pixels) where the piece fits.
///
/// Input: bytes of an image containing BOTH the background (top) and piece (bottom)
/// stacked vertically, OR a single background image where the notch is visible.
///
/// For a single image with a visible notch/shadow, we detect the notch position.
pub fn solve(image_bytes: &[u8]) -> Result<String> {
    let img = image::load_from_memory(image_bytes)?;
    let offset = find_notch_offset(&img)?;
    Ok(offset.to_string())
}

/// Find the X offset of a puzzle notch in the image.
/// Strategy: look for a region with high edge density that forms a square-ish shape.
fn find_notch_offset(img: &DynamicImage) -> Result<u32> {
    let gray = img.to_luma8();
    let (w, h) = gray.dimensions();

    // Compute vertical edge magnitude per column (Sobel-like horizontal gradient)
    let mut col_edge: Vec<u64> = vec![0; w as usize];
    for y in 0..h {
        for x in 1..w - 1 {
            let left = gray.get_pixel(x - 1, y).0[0] as i32;
            let right = gray.get_pixel(x + 1, y).0[0] as i32;
            let diff = (right - left).unsigned_abs() as u64;
            col_edge[x as usize] += diff;
        }
    }

    // The notch/shadow creates a spike in edge energy.
    // Find the column with maximum edge energy in the middle 80% of the image
    // (notch is rarely at the very edge).
    let start = (w as f32 * 0.1) as usize;
    let end = (w as f32 * 0.9) as usize;

    let (max_col, _max_val) = col_edge[start..end]
        .iter()
        .enumerate()
        .max_by_key(|(_, v)| **v)
        .context("empty image")?;

    Ok((start + max_col) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slide_basic() {
        // Create a 200x100 gray image with a dark vertical stripe at x=80
        let mut img = image::GrayImage::new(200, 100);
        for y in 0..100 {
            for x in 0..200u32 {
                let val = if (78..=82).contains(&x) { 30 } else { 200 };
                img.put_pixel(x, y, image::Luma([val]));
            }
        }
        let dyn_img = DynamicImage::ImageLuma8(img);
        let offset = find_notch_offset(&dyn_img).unwrap();
        // Should be near 80 (±5)
        assert!((75..=85).contains(&offset), "got offset {offset}");
    }
}
