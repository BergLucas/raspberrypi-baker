use std::env;
use std::fs::File;
use std::io;
use std::path::PathBuf;

use crate::images::parsing;
use crate::images::BakerImage;
use chrono::NaiveDateTime;
use regex::Regex;
use serde::{Deserialize, Serialize};
use url::Url;

fn list_raspios_image_names(
    registry: &str,
) -> Result<Vec<(String, NaiveDateTime)>, Box<dyn std::error::Error>> {
    let body = reqwest::blocking::get(&format!(
        "https://downloads.raspberrypi.org/{}/images/",
        registry
    ))?
    .text()?;

    Ok(parsing::parse_apache_directory_listing(&body)?
        .into_iter()
        .filter(|file| file.is_directory())
        .map(|file| (file.name().to_string(), file.last_modified()))
        .collect())
}

fn list_raspios_repositories() -> Result<Vec<(String, NaiveDateTime)>, Box<dyn std::error::Error>> {
    let body = reqwest::blocking::get("https://downloads.raspberrypi.org/")?.text()?;

    Ok(parsing::parse_apache_directory_listing(&body)?
        .into_iter()
        .filter(|file| file.is_directory() && file.name().starts_with("raspios"))
        .map(|file| (file.name().to_string(), file.last_modified()))
        .collect())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadableBakerImage {
    url: String,
    image: BakerImage,
}

impl DownloadableBakerImage {
    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn image(&self) -> &BakerImage {
        &self.image
    }
}

fn get_raspios_images(
    registry: &str,
    image_name: &str,
) -> Result<DownloadableBakerImage, Box<dyn std::error::Error>> {
    let body = reqwest::blocking::get(&format!(
        "https://downloads.raspberrypi.org/{}/images/{}/",
        registry, image_name
    ))?
    .text()?;

    let files: Vec<String> = parsing::parse_apache_directory_listing(&body)?
        .into_iter()
        .filter(|file| !file.is_directory())
        .map(|file| file.name().to_string())
        .collect();

    let filename = files
        .iter()
        .find(|file| file.ends_with(".zip") || file.ends_with(".xz"))
        .ok_or("No image url found")?;

    let sha256_url = files
        .iter()
        .find(|file| file.ends_with(".sha256"))
        .ok_or("No sha256 url found")?;

    let sha256 = reqwest::blocking::get(&format!(
        "https://downloads.raspberrypi.org/{}/images/{}/{}",
        registry, image_name, sha256_url
    ))?
    .text()?
    .split_whitespace()
    .next()
    .ok_or("No sha256 found")?
    .to_string();

    let (name, tag, platform) =
        match Regex::new(r"(\d{4}-\d{2}-\d{2})-(\w+)-(\w+)-(\w+)(?:-(\w+))?")?
            .captures(filename)
            .ok_or("Invalid filename")?
            .iter()
            .collect::<Vec<_>>()
            .as_slice()
        {
            [Some(_), Some(date), Some(name), Some(version), Some(platform), feature] => {
                let date_concat = date.as_str().replace("-", "");
                let tag = match feature {
                    Some(feature) => {
                        format!("{}-{}-{}", version.as_str(), date_concat, feature.as_str())
                    }
                    None => format!("{}-{}", version.as_str(), date_concat),
                };
                (name.as_str(), tag, platform.as_str())
            }
            _ => {
                return Err("Invalid image file".into());
            }
        };

    let url = format!(
        "https://downloads.raspberrypi.org/{}/images/{}/{}",
        registry, image_name, filename
    );

    Ok(DownloadableBakerImage {
        url,
        image: BakerImage {
            platform: platform.to_string(),
            name: name.to_string(),
            tag,
            sha256,
        },
    })
}

fn list_raspios_images_from_repository(
    repository: String,
    date: Option<NaiveDateTime>,
) -> Result<impl Iterator<Item = DownloadableBakerImage>, Box<dyn std::error::Error>> {
    Ok(list_raspios_image_names(&repository)?
        .into_iter()
        .filter(move |(_, last_modified)| date.map_or(true, |date| date <= *last_modified))
        .flat_map(move |(name, _)| get_raspios_images(&repository, &name)))
}

pub fn list_raspios_images(
    date: Option<NaiveDateTime>,
) -> Result<impl Iterator<Item = DownloadableBakerImage>, Box<dyn std::error::Error>> {
    Ok(list_raspios_repositories()?
        .into_iter()
        .filter(move |(_, last_modified)| date.map_or(true, |date| date <= *last_modified))
        .flat_map(move |(name, _)| list_raspios_images_from_repository(name, date))
        .flatten())
}

pub fn download_image(
    image_dir: PathBuf,
    downloadable_image: &DownloadableBakerImage,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder().timeout(None).build()?;

    let url = Url::parse(downloadable_image.url())?;

    let filename = url
        .path_segments()
        .ok_or("Invalid url")?
        .last()
        .ok_or("Invalid filename")?;

    let mut response = client.get(url.clone()).send()?;

    let temp_filepath = env::temp_dir().join(filename);
    let mut temp_file = File::create(&temp_filepath)?;
    response.copy_to(&mut temp_file)?;
    temp_file.sync_data()?;

    let mut file =
        File::create(image_dir.join(format!("{}.img", downloadable_image.image().sha256())))?;

    if filename.ends_with(".zip") {
        let mut archive = zip::ZipArchive::new(&temp_file)?;
        let mut image_file = archive.by_index(0)?;
        io::copy(&mut image_file, &mut file)?;
    } else if filename.ends_with(".xz") {
        let mut archive = xz2::read::XzDecoder::new(File::open(&temp_filepath)?);
        io::copy(&mut archive, &mut file)?;
    } else {
        return Err("Invalid image file".into());
    }

    file.sync_data()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_raspios_registries() {
        let registries = list_raspios_repositories()
            .unwrap()
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<String>>();

        assert!(registries.contains(&"raspios_arm64".to_string()));
        assert!(registries.contains(&"raspios_armhf".to_string()));
        assert!(registries.contains(&"raspios_full_arm64".to_string()));
        assert!(registries.contains(&"raspios_full_armhf".to_string()));
        assert!(registries.contains(&"raspios_lite_arm64".to_string()));
        assert!(registries.contains(&"raspios_lite_armhf".to_string()));
        assert!(registries.contains(&"raspios_oldstable_arm64".to_string()));
        assert!(registries.contains(&"raspios_oldstable_armhf".to_string()));
        assert!(registries.contains(&"raspios_oldstable_lite_arm64".to_string()));
        assert!(registries.contains(&"raspios_oldstable_lite_armhf".to_string()));
        assert!(registries.contains(&"raspios_oldstable_full_arm64".to_string()));
        assert!(registries.contains(&"raspios_oldstable_full_armhf".to_string()));
    }
}
