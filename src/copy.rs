use std::{fs, path::PathBuf};

use crate::mount::MountedImage;
use path_absolutize::*;

impl MountedImage {
    pub fn copy(
        &self,
        label: &str,
        source: &PathBuf,
        target: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mount_point = self.get_mount_point(label)?;

        let mount_point_string = mount_point
            .to_str()
            .ok_or("Failed to convert path to string")?;

        let target_str = target.to_str().ok_or("Failed to convert path to string")?;

        let mounted_target = PathBuf::from(mount_point_string.to_string() + "/" + target_str);

        let absolute_mounted_target = mounted_target.absolutize()?;

        if !absolute_mounted_target.starts_with(mount_point) {
            return Err("Invalid target path".into());
        }

        fs::copy(source, absolute_mounted_target)?;

        Ok(())
    }
}
