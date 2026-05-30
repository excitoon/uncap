use anyhow::{bail, Context, Result};
use image::GrayImage;
use imageproc::contrast::{threshold, ThresholdType};
use std::process::Command;

/// Solve a math-expression captcha (e.g. "3 + 7 = ?", "12 - 4 = ?").
/// OCRs the expression, evaluates it, returns the numeric answer.
pub fn solve(image_bytes: &[u8]) -> Result<String> {
    let img = image::load_from_memory(image_bytes)?;
    let processed = preprocess(&img);
    let raw = ocr_math(&processed)?;
    evaluate(&raw)
}

fn preprocess(img: &image::DynamicImage) -> GrayImage {
    let gray = img.to_luma8();
    let (w, h) = gray.dimensions();
    let scaled =
        image::imageops::resize(&gray, w * 2, h * 2, image::imageops::FilterType::Triangle);
    let mean =
        scaled.pixels().map(|p| p.0[0] as u64).sum::<u64>() / (scaled.pixels().count() as u64);
    let bin = threshold(&scaled, mean as u8, ThresholdType::Binary);

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

fn ocr_math(img: &GrayImage) -> Result<String> {
    let tmp = tempfile::NamedTempFile::with_suffix(".png")?;
    img.save(tmp.path()).context("save temp image")?;

    let output = Command::new("tesseract")
        .arg(tmp.path())
        .arg("stdout")
        .arg("--psm")
        .arg("7")
        .arg("-c")
        .arg("tessedit_char_whitelist=0123456789+-*/x×=? ")
        .output()
        .context("failed to run tesseract")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("tesseract failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Parse and evaluate a simple math expression like "3 + 7", "12 - 4", "5 * 3", "10 / 2".
/// Also handles "3 + 7 = ?" format.
fn evaluate(expr: &str) -> Result<String> {
    // Strip trailing "= ?" or "="
    let expr = expr.replace("=", " ").replace("?", " ");
    let expr = expr.trim();

    // Tokenize: number op number
    let parts: Vec<&str> = expr.split_whitespace().collect();

    if parts.len() < 3 {
        // Try to find pattern: digits, operator, digits
        let re = regex::Regex::new(r"(\d+)\s*([+\-*/x×])\s*(\d+)")?;
        if let Some(caps) = re.captures(expr) {
            let a: i64 = caps[1].parse()?;
            let op = &caps[2];
            let b: i64 = caps[3].parse()?;
            return Ok(compute(a, op, b)?.to_string());
        }
        bail!("cannot parse math expression: {expr:?}");
    }

    let a: i64 = parts[0].parse().context("parse left operand")?;
    let op = parts[1];
    let b: i64 = parts[2].parse().context("parse right operand")?;

    Ok(compute(a, op, b)?.to_string())
}

fn compute(a: i64, op: &str, b: i64) -> Result<i64> {
    match op {
        "+" => Ok(a + b),
        "-" => Ok(a - b),
        "*" | "x" | "×" => Ok(a * b),
        "/" => {
            if b == 0 {
                bail!("division by zero");
            }
            Ok(a / b)
        }
        _ => bail!("unknown operator: {op:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate() {
        assert_eq!(evaluate("3 + 7").unwrap(), "10");
        assert_eq!(evaluate("12 - 4").unwrap(), "8");
        assert_eq!(evaluate("5 * 3").unwrap(), "15");
        assert_eq!(evaluate("10 / 2").unwrap(), "5");
        assert_eq!(evaluate("3 + 7 = ?").unwrap(), "10");
        assert_eq!(evaluate("6x4").unwrap(), "24");
    }
}
