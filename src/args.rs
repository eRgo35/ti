use clap::Parser;

const NAME: Option<&str> = option_env!("CARGO_PKG_NAME");
const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
const DESCRIPTION: Option<&str> = option_env!("CARGO_PKG_DESCRIPTION");

#[derive(Parser, Debug)]
#[command(
    name = NAME.unwrap_or("ti"),
    author = "Michał Czyż",
    version = VERSION.unwrap_or("unknown"),
    about = DESCRIPTION.unwrap_or("A simple terminal timer"),
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

    #[arg(long, default_value_t = false, help = "Clear cache")]
    pub clear: bool,

    pub time: Option<String>,
}
