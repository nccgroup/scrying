use clap::Clap;
use std::str::FromStr;

#[derive(PartialEq, Debug)]
pub enum Mode {
    Web,
    Rdp,
}

impl FromStr for Mode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Mode::{Rdp, Web};
        match s {
            "web" => Ok(Web),
            "rdp" => Ok(Rdp),
            _ => Err("Mode must be \"web\" or \"rdp\""),
        }
    }
}

#[derive(Clap, Debug)]
#[clap(version = "0.1", author = "David Y. <david.young@nccgroup.com>")]
pub struct Opts {
    #[clap(short, long)]
    pub input: Option<String>,

    #[clap(short, long)]
    pub target: Option<String>,

    #[clap(short, long)]
    pub mode: Mode,

    #[clap(short, long, default_value = "output")]
    pub output_dir: String,

    #[clap(short, long, parse(from_occurrences))]
    pub verbose: i32,
}

pub fn parse() -> Opts {
    Opts::parse()
}
