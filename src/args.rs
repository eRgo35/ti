use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short = 'H', long, default_value_t = 0)]
    pub hours: u64,

    #[arg(short = 'M', long, default_value_t = 0)]
    pub minutes: u64,

    #[arg(short = 'S', long, default_value_t = 0)]
    pub seconds: u64,

    #[arg(long, default_value = "/usr/share/ti/ANSI_Mono.flf")]
    pub font: PathBuf,

    #[arg(long, default_value = "/tmp/ti_countdown.tmp")]
    pub cache: PathBuf,
}
