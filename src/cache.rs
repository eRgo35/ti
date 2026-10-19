use std::{
    env::temp_dir,
    fs::{read_to_string, remove_file, write},
    path::PathBuf,
};

/// Best-effort persistence of the remaining seconds. IO failures are never
/// fatal: the timer works the same without a usable cache.
pub struct Cache {
    cache_file: PathBuf,
}

impl Cache {
    pub fn new(cache_file: String) -> Self {
        let cache_file = if cache_file.is_empty() {
            temp_dir().join("ti_countdown.tmp")
        } else {
            PathBuf::from(cache_file)
        };

        Self { cache_file }
    }

    pub fn clear(&self) {
        let _ = remove_file(&self.cache_file);
    }

    pub fn save(&self, countdown: u64) {
        let _ = write(&self.cache_file, countdown.to_string());
    }

    pub fn load(&self) -> Option<u64> {
        read_to_string(&self.cache_file).ok()?.trim().parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_cache(name: &str) -> Cache {
        let path = temp_dir().join(format!("ti_test_{}_{}", std::process::id(), name));
        let _ = fs::remove_file(&path);
        Cache::new(path.to_string_lossy().into_owned())
    }

    #[test]
    fn save_then_load_roundtrip() {
        let cache = temp_cache("roundtrip");
        cache.save(42);
        assert_eq!(cache.load(), Some(42));
        cache.save(7);
        assert_eq!(cache.load(), Some(7));
        cache.clear();
    }

    #[test]
    fn save_truncates_longer_previous_content() {
        let cache = temp_cache("truncate");
        cache.save(123456);
        cache.save(9);
        assert_eq!(cache.load(), Some(9));
        cache.clear();
    }

    #[test]
    fn clear_removes_file() {
        let cache = temp_cache("clear");
        cache.save(5);
        cache.clear();
        assert_eq!(cache.load(), None);
        assert!(!cache.cache_file.exists());
        // Clearing a missing file is fine.
        cache.clear();
    }

    #[test]
    fn load_missing_is_none() {
        assert_eq!(temp_cache("missing").load(), None);
    }

    #[test]
    fn load_ignores_garbage() {
        let cache = temp_cache("garbage");
        fs::write(&cache.cache_file, "not a number").unwrap();
        assert_eq!(cache.load(), None);
        fs::write(&cache.cache_file, "-3").unwrap();
        assert_eq!(cache.load(), None);
        fs::write(&cache.cache_file, "").unwrap();
        assert_eq!(cache.load(), None);
        cache.clear();
    }

    #[test]
    fn load_trims_whitespace() {
        let cache = temp_cache("trim");
        fs::write(&cache.cache_file, " 12\n").unwrap();
        assert_eq!(cache.load(), Some(12));
        cache.clear();
    }

    #[test]
    fn unwritable_path_does_not_panic() {
        let path = temp_dir()
            .join(format!("ti_test_{}_nonexistent_dir", std::process::id()))
            .join("c");
        let cache = Cache::new(path.to_string_lossy().into_owned());
        cache.save(5);
        assert_eq!(cache.load(), None);
        cache.clear();
    }
}
