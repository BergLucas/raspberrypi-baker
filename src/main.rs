use clap::{Parser, Subcommand};
pub mod parsing;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Build an image from a Bakerfile")]
    Build {
        path: String,

        #[arg(short, long)]
        file: Option<String>,

        #[arg(short, long)]
        output: Option<String>,

        #[arg(short, long)]
        tag: Option<String>,
    },
    #[command(about = "List images")]
    Images { image: String },
    #[command(about = "Remove an image")]
    Rmi { image: String },
    #[command(about = "Burn an image to a device")]
    Burn { device_file: String, image: String },
}

fn main() {
    let args = Cli::parse();
}
