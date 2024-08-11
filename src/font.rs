use figlet_rs::FIGfont;

pub fn load_font(font: String) -> FIGfont {
    let default_font = FIGfont::standard().unwrap();
    let ansi_mono = std::include_str!("ANSI_Mono.flf");
    let ansi_mono_font = FIGfont::from_content(ansi_mono).unwrap_or(default_font);

    FIGfont::from_file(&font).unwrap_or(ansi_mono_font)
}
