pub mod audio;
pub mod math;
pub mod slide;
pub mod text;

use anyhow::Result;

/// The kind of captcha to solve.
#[derive(Debug, Clone, Copy)]
pub enum SolverKind {
    /// Image with distorted text — OCR.
    Text,
    /// Image with a math expression (e.g. "3 + 7 = ?").
    Math,
    /// Audio captcha — speech-to-text.
    Audio,
    /// Slide/jigsaw puzzle — find the offset.
    Slide,
}

/// Solve captcha bytes with the given strategy.
pub fn solve(bytes: &[u8], kind: SolverKind) -> Result<String> {
    match kind {
        SolverKind::Text => text::solve(bytes),
        SolverKind::Math => math::solve(bytes),
        SolverKind::Audio => audio::solve(bytes),
        SolverKind::Slide => slide::solve(bytes),
    }
}
