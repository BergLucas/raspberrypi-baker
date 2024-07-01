use crate::images::{download::download_image, fetch::fetch_baker_images};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

mod download;
mod fetch;
mod parsing;
mod repository;

fn get_images_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(crate::get_app_dir()?.join("images"))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BakerImage {
    platform: String,
    name: String,
    tag: String,
    sha256: String,
}

impl BakerImage {
    pub fn platform(&self) -> &str {
        &self.platform
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn tag(&self) -> &str {
        &self.tag
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }
    pub fn path(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        Ok(get_images_dir()?.join(format!("{}.img", self.sha256)))
    }
}

pub fn list() -> Result<Vec<BakerImage>, Box<dyn std::error::Error>> {
    repository::read_repository().or_else(|_| Ok(Vec::new()))
}

pub fn pull(
    platform: &str,
    name: &str,
    tag: &str,
) -> Result<BakerImage, Box<dyn std::error::Error>> {
    let mut images = list()?;

    let image = images
        .iter()
        .find(|image| image.platform() == platform && image.name() == name && image.tag() == tag);

    match image {
        Some(image) => Ok(image.clone()),
        None => {
            let downloadable_image = fetch_baker_images()?
                .into_iter()
                .find(|downloadable_image| {
                    let image = downloadable_image.image();
                    image.platform() == platform && image.name() == name && image.tag() == tag
                })
                .ok_or("Image not found")?;

            let image = downloadable_image.image();

            println!("Downloading image: {}", image.full_name());

            download_image(image.path()?, &downloadable_image)?;

            images.push(image.clone());

            repository::write_repository(&images)?;

            Ok(image.clone())
        }
    }
}

pub fn rmi(platform: &str, name: &str, tag: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut images: Vec<BakerImage> = Vec::new();

    for image in list()? {
        if image.platform() == platform && image.name() == name && image.tag() == tag {
            fs::remove_file(image.path()?)?;
        } else {
            images.push(image.clone());
        }
    }

    repository::write_repository(&images)?;

    Ok(())
}
