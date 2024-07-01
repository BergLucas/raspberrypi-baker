use std::fs;
use std::{fs::File, path::PathBuf};

use crate::get_app_dir;
use crate::images::BakerImage;

fn get_repository_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(get_app_dir()?.join("repositories.json"))
}

pub fn read_repository() -> Result<Vec<BakerImage>, Box<dyn std::error::Error>> {
    Ok(serde_json::from_reader(
        File::open(get_repository_path()?)?,
    )?)
}

pub fn write_repository(images: &[BakerImage]) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(
        get_repository_path()?
            .parent()
            .ok_or("Invalid repository path")?,
    )?;

    serde_json::to_writer_pretty(File::create(get_repository_path()?)?, images)?;

    Ok(())
}
