use crate::{
    images::{download::download_image, fetch::fetch_baker_images},
    mount::MountedImage,
    parsing::parser::{self, BakerFile},
};
use glob::glob;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

mod download;
mod fetch;
mod hash;
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

pub fn build(
    file: PathBuf,
    name: Option<String>,
    tag: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open(&file)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    let (_, bakerfile) = parser::parse_baker_file::<()>(&contents)?;
    let from = bakerfile.from;
    let platform = from.platform.unwrap_or("arm64".into());
    let image = pull(
        &platform.clone(),
        &from.image,
        &from.tag.ok_or("Image tag is required")?,
    )?;

    // Copy image into a temporary file
    let image_path = image.path()?;
    let tmp_dir = tempdir::TempDir::new("baker")?;
    let tmp_path = tmp_dir.path().join("i_love_bakery.img");
    fs::copy(image_path, &tmp_path)?;

    // Mount image
    let mounted = MountedImage::new(&tmp_path)?;

    // Init environment
    let mut user = "root".to_string();
    let mut workdir = "/".to_string();
    let mut envs: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // Apply instructions
    for instruction in bakerfile.instructions {
        match instruction {
            parser::Instruction::USER(u) => user = u,
            parser::Instruction::WORKDIR(w) => workdir = w,
            parser::Instruction::ENV(e) => envs.extend(e),
            parser::Instruction::RUN(r) => {
                mounted.run(
                    mounted.labels().last().ok_or("No label found")?,
                    crate::run::RunEnvironment::SystemdNspawn,
                    &envs,
                    &user,
                    &workdir,
                    &r,
                )?;
            }
            parser::Instruction::COPY(sources, dest) => {
                for source in glob(&sources)?.collect::<Result<Vec<_>, _>>()? {
                    mounted.copy(
                        mounted.labels().last().ok_or("No label found")?,
                        &source,
                        &dest,
                    )?;
                }
            }
            _ => {
                println!("Skipping Instruction {:?}: Not implemented", instruction);
            }
        }
    }
    // Unmount image and save it
    mounted.unmount()?;
    let img_dir = get_images_dir()?;
    let digest = hash::sha256_digest(&tmp_path)?;
    let dest_path = img_dir.join(digest.clone() + ".img");
    fs::copy(&tmp_path, dest_path)?;

    // Update repository
    let mut repos = repository::read_repository()?;
    repos.push(BakerImage {
        platform,
        name: name.unwrap_or(digest.clone()),
        tag: tag.unwrap_or("latest".into()),
        sha256: digest,
    });

    repository::write_repository(&repos)?;
    Ok(())
}
