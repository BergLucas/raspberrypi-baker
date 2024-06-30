use crate::get_app_dir;
use crate::images::download::{list_raspios_images, DownloadableBakerImage};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json;
use std::fs::File;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

fn get_downloadable_images_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(get_app_dir()?.join("downloadable-images.json"))
}

pub fn fetch_baker_images() -> Result<Vec<DownloadableBakerImage>, Box<dyn std::error::Error>> {
    let downloadable_images_dir = get_downloadable_images_path()?;

    let (mut downloadable_images, date): (Vec<DownloadableBakerImage>, Option<NaiveDateTime>) =
        match File::open(downloadable_images_dir) {
            Ok(file) => {
                let date: DateTime<Utc> = file.metadata()?.modified()?.into();
                (serde_json::from_reader(file)?, Some(date.naive_utc()))
            }
            Err(_) => (Vec::new(), None),
        };

    for downloadable_image in list_raspios_images(date)? {
        let image = downloadable_image.image();
        println!(
            "Fetching {:?} for {:?}",
            image.full_name(),
            image.platform()
        );
        downloadable_images.push(downloadable_image);
        sleep(Duration::from_millis(500));
    }

    serde_json::to_writer_pretty(
        File::create(get_downloadable_images_path()?)?,
        &downloadable_images,
    )?;

    Ok(downloadable_images)
}
