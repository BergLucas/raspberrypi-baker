use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

mod images;

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
    #[command(about = "Pull an image")]
    Pull {
        #[arg(value_name = "NAME:TAG")]
        image: String,

        #[arg(short, long)]
        platform: Option<String>,
    },
    #[command(about = "List images")]
    Images {},
    #[command(about = "Remove an image")]
    Rmi { image: String },
    #[command(about = "Burn an image to a device")]
    Burn { device_file: String, image: String },
}

fn get_app_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let app_dir = dirs::config_local_dir()
        .ok_or("Invalid config local directory")?
        .join("raspberrypi-baker");

    fs::create_dir_all(&app_dir)?;

    Ok(app_dir)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    match args.command {
        Commands::Pull { image, platform } => {
            let platform = platform.unwrap_or("arm64".to_string());
            match image.split(":").collect::<Vec<&str>>().as_slice() {
                [name, tag] => images::pull(&platform, name, tag),
                _ => Err("Invalid image name".into()),
            }?;
            Ok(())
        }
        Commands::Images {} => {
            println!("{:<15} {:<30} {:<64}", "Repository", "Tag", "SHA256");
            for image in images::list()? {
                println!(
                    "{:<15} {:<30} {:<64}",
                    image.name(),
                    image.tag(),
                    image.sha256()
                );
            }
            Ok(())
        }
        Commands::Rmi { image } => {
            let platform = "arm64";
            match image.split(":").collect::<Vec<&str>>().as_slice() {
                [name, tag] => images::rmi(platform, name, tag),
                _ => Err("Invalid image name".into()),
            }
        }
        _ => unimplemented!(),
    }
}
