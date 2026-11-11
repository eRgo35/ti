use std::panic::{self, AssertUnwindSafe};

use figlet_rs::FIGlet;

const ANSI_MONO: &str = include_str!("../resources/ANSI_Mono.flf");

/// Loads the font at `path`; an empty path selects the embedded default.
/// An explicitly requested font that cannot be loaded is an error rather than
/// being silently replaced.
pub fn load_font(path: &str) -> Result<FIGlet, String> {
    if path.is_empty() {
        return FIGlet::from_content(ANSI_MONO).or_else(|_| FIGlet::standard());
    }

    // The figlet-rs parser can still panic on some malformed headers (for
    // example a zero height) instead of returning an error.
    let content =
        std::fs::read_to_string(path).map_err(|err| format!("cannot read font '{path}': {err}"))?;

    quietly_catch_panic(|| FIGlet::from_content(&content))
        .unwrap_or_else(|| Err("malformed font file".to_string()))
        .and_then(require_clock_glyphs)
        .map_err(|err| format!("invalid font '{path}': {err}"))
}

/// figlet-rs loads fonts with missing glyphs and silently skips them when
/// rendering, so a truncated file would draw a broken clock.
fn require_clock_glyphs(font: FIGlet) -> Result<FIGlet, String> {
    match "0123456789:"
        .chars()
        .find(|c| !font.fonts.contains_key(&u32::from(*c)))
    {
        Some(missing) => Err(format!("no glyph for '{missing}'")),
        None => Ok(font),
    }
}

/// Runs `f`, returning `None` if it panicked, without printing the panic
/// message. Only meant for use before the terminal is in raw mode.
fn quietly_catch_panic<T>(f: impl FnOnce() -> T) -> Option<T> {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(AssertUnwindSafe(f));
    panic::set_hook(previous);
    result.ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_font(name: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("ti_test_{}_{}", std::process::id(), name));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn empty_path_loads_embedded_default() {
        let font = load_font("").unwrap();
        assert!(font.convert("00:00:00").is_some());
    }

    #[test]
    fn missing_file_is_error() {
        let err = load_font("/definitely/not/a/font.flf").unwrap_err();
        assert!(err.contains("/definitely/not/a/font.flf"), "{err}");
        assert!(err.contains("No such file"), "{err}");
    }

    #[test]
    fn garbage_file_is_error() {
        let path = temp_font("garbage_font", "this is not a figlet font");
        let res = load_font(&path.to_string_lossy());
        let _ = std::fs::remove_file(&path);
        assert!(res.is_err());
    }

    #[test]
    fn truncated_font_is_error_not_panic() {
        let header = "flf2a$ 1 1 4 0 1\ncomment\n";
        let glyphs: String = (32u8..48)
            .map(|c| format!("{}@@\n", if c == 32 { '$' } else { c as char }))
            .collect();
        let path = temp_font("truncated_font", &format!("{header}{glyphs}"));
        let res = load_font(&path.to_string_lossy());
        let _ = std::fs::remove_file(&path);
        assert!(res.unwrap_err().contains("no glyph for '0'"));
    }

    #[test]
    fn zero_height_font_is_error_not_panic() {
        let path = temp_font("zero_height_font", "flf2a$ 0 0 4 0 0\nx@\n");
        let res = load_font(&path.to_string_lossy());
        let _ = std::fs::remove_file(&path);
        assert!(res.unwrap_err().contains("malformed font file"));
    }
}
