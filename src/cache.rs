use std::{
    env::temp_dir,
    fs::{remove_file, File},
    io::{Read, Write},
    path::PathBuf,
};

#[derive(Clone, Debug)]
pub struct Cache {
    pub cache_file: PathBuf,
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
        let state = format!("{}", countdown);
        let mut file =
            File::create_new(&self.cache_file).unwrap_or(File::create(&self.cache_file).unwrap());
        file.write_all(state.as_bytes()).unwrap();
    }

    pub fn load(&self) -> Option<u64> {
        let mut file = File::open(&self.cache_file).ok()?;
        let mut countdown = String::new();
        file.read_to_string(&mut countdown).ok()?;

        let countdown = countdown.trim().parse::<u64>().ok()?;

        Some(countdown)
    }
}
