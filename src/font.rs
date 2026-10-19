use std::panic::{self, AssertUnwindSafe};

use figlet_rs::FIGfont;

const ANSI_MONO: &str = include_str!("../resources/ANSI_Mono.flf");

/// Loads the font at `path`; an empty path selects the embedded default.
/// An explicitly requested font that cannot be loaded is an error rather than
/// being silently replaced.
pub fn load_font(path: &str) -> Result<FIGfont, String> {
    if path.is_empty() {
        return FIGfont::from_content(ANSI_MONO).or_else(|_| FIGfont::standard());
    }

    // figlet-rs indexes and subtracts without bounds checks, so a truncated
    // font file panics inside the parser instead of returning an error.
    let content =
        std::fs::read_to_string(path).map_err(|err| format!("cannot read font '{path}': {err}"))?;

    quietly_catch_panic(|| FIGfont::from_content(&content))
        .unwrap_or_else(|| Err("malformed font file".to_string()))
        .map_err(|err| format!("invalid font '{path}': {err}"))
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
        assert!(res.is_err());
    }
}
