use anyhow::{bail, Context, Result};
use std::process::Command;

/// Solve an audio captcha. Expects audio bytes (wav/mp3/ogg).
/// Uses whisper CLI or similar speech-to-text tool.
/// Falls back to pocketsphinx if available.
pub fn solve(audio_bytes: &[u8]) -> Result<String> {
    // Write audio to temp file
    let suffix = detect_format(audio_bytes);
    let tmp = tempfile::NamedTempFile::with_suffix(suffix)?;
    std::fs::write(tmp.path(), audio_bytes)?;

    // Try whisper first (OpenAI Whisper CLI)
    if let Ok(text) = try_whisper(tmp.path()) {
        return Ok(text);
    }

    // Fall back to macOS `say` + speech recognition isn't possible,
    // so try ffmpeg + vosk or just report the error clearly.
    bail!(
        "audio captcha solving requires 'whisper' CLI (pip install openai-whisper). \
         Install it and try again."
    )
}

fn detect_format(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"RIFF") {
        ".wav"
    } else if bytes.len() > 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0 {
        ".mp3"
    } else if bytes.starts_with(b"OggS") {
        ".ogg"
    } else {
        ".wav"
    }
}

fn try_whisper(path: &std::path::Path) -> Result<String> {
    let output = Command::new("whisper")
        .arg(path)
        .arg("--model")
        .arg("base")
        .arg("--output_format")
        .arg("txt")
        .arg("--output_dir")
        .arg(path.parent().unwrap_or(std::path::Path::new("/tmp")))
        .output()
        .context("whisper not found")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("whisper failed: {stderr}");
    }

    // Whisper writes a .txt file alongside the audio
    let txt_path = path.with_extension("txt");
    let text = std::fs::read_to_string(&txt_path)
        .context("read whisper output")?
        .trim()
        .to_string();

    // Clean up
    let _ = std::fs::remove_file(&txt_path);

    // Extract just alphanumeric characters (captchas are usually short codes)
    let cleaned: String = text.chars().filter(|c| c.is_alphanumeric()).collect();

    Ok(cleaned)
}
