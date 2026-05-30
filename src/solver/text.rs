use anyhow::{bail, Context, Result};
use image::GrayImage;
use imageproc::contrast::{threshold, ThresholdType};
use std::process::Command;

/// Solve a text-based image captcha via preprocessing + Tesseract OCR.
pub fn solve(image_bytes: &[u8]) -> Result<String> {
    let img = image::load_from_memory(image_bytes)?;
    let processed = preprocess(&img);
    recognize(&processed)
}

fn preprocess(img: &image::DynamicImage) -> GrayImage {
    let gray = img.to_luma8();

    // Scale up 2× for better OCR on small text
    let (w, h) = gray.dimensions();
    let scaled =
        image::imageops::resize(&gray, w * 2, h * 2, image::imageops::FilterType::Triangle);

    // Mean threshold
    let mean =
        scaled.pixels().map(|p| p.0[0] as u64).sum::<u64>() / (scaled.pixels().count() as u64);
    let bin = threshold(&scaled, mean as u8, ThresholdType::Binary);

    // Auto-invert if background is dark
    let black_count = bin.pixels().filter(|p| p.0[0] == 0).count();
    if black_count > bin.pixels().count() / 2 {
        let mut inverted = bin;
        for p in inverted.pixels_mut() {
            p.0[0] = 255 - p.0[0];
        }
        inverted
    } else {
        bin
    }
}

fn recognize(img: &GrayImage) -> Result<String> {
    let tmp = tempfile::NamedTempFile::with_suffix(".png")?;
    img.save(tmp.path()).context("save temp image")?;

    let output = Command::new("tesseract")
        .arg(tmp.path())
        .arg("stdout")
        .arg("--psm")
        .arg("7")
        .arg("-c")
        .arg("tessedit_char_whitelist=0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .output()
        .context("failed to run tesseract — is it installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("tesseract failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
