use clap::Parser;

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(
    name = "ti",
    author = "Michał Czyż",
    version = VERSION.unwrap_or("unknown"),
    about = "A simple terminal timer",
    long_about = None
)]
pub struct Args {
    #[arg(short = 'H', long, default_value_t = 0, help = "Hours")]
    pub hours: u64,

    #[arg(
        short_alias = 'm',
        short = 'M',
        long,
        default_value_t = 0,
        help = "Minutes"
    )]
    pub minutes: u64,

    #[arg(
        short_alias = 's',
        short = 'S',
        long,
        default_value_t = 0,
        help = "Seconds"
    )]
    pub seconds: u64,

    #[arg(long, default_value_t = String::from(""), help = "Path to custom font file")]
    pub font: String,

    #[arg(long, default_value_t = String::from(""), help = "Path to cache file")]
    pub cache: String,
}
