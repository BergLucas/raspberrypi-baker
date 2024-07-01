use crate::images::BakerImage;
use glob::glob;
use loopdev::{LoopControl, LoopDevice};
use path_absolutize::*;
use std::{collections::BTreeMap, fs, path::PathBuf, thread::sleep, time::Duration};
use sys_mount::{Mount, Unmount, UnmountFlags};
use tempdir::TempDir;
use udev::Device;

pub struct MountedBakerImage {
    loop_device: LoopDevice,
    mount_dir: TempDir,
    mount_points: BTreeMap<String, Mount>,
}

impl MountedBakerImage {
    pub fn new(image: BakerImage) -> Result<MountedBakerImage, Box<dyn std::error::Error>> {
        let loop_control = LoopControl::open()?;

        let loop_device = loop_control.next_free()?;

        loop_device
            .with()
            .part_scan(true)
            .attach(image.path()?.as_path())?;

        let loop_device_path = loop_device.path().ok_or("Invalid loop device path")?;

        let partition_devices_pattern = loop_device_path
            .to_str()
            .ok_or("Failed to convert path to string")?
            .to_string()
            + "*";

        let partition_devices = glob(&partition_devices_pattern)?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|dev| *dev != loop_device_path)
            .collect::<Vec<_>>();

        let mount_dir = TempDir::new("baker")?;

        let mount_points = partition_devices
            .into_iter()
            .map(|partition_device| {
                let sysname = partition_device
                    .file_name()
                    .ok_or("Invalid device path")?
                    .to_str()
                    .ok_or("Failed to convert path to string")?
                    .to_string();

                let device = Device::from_subsystem_sysname("block".into(), sysname)?;

                while !device.is_initialized() {
                    sleep(Duration::from_millis(100));
                }

                let label = device
                    .property_value("ID_FS_LABEL_ENC")
                    .ok_or("Failed to get device label")?
                    .to_str()
                    .ok_or("Failed to convert label to string")?
                    .to_string();

                let mount_point = mount_dir.path().join(&label);

                fs::create_dir_all(mount_point.as_path())?;

                Ok((label, Mount::new(partition_device, mount_point)?))
            })
            .collect::<Result<BTreeMap<String, Mount>, Box<dyn std::error::Error>>>()?;

        Ok(MountedBakerImage {
            loop_device,
            mount_dir,
            mount_points,
        })
    }
    pub fn labels(&self) -> Vec<String> {
        self.mount_points.keys().cloned().collect()
    }
    pub fn copy(
        &self,
        label: &str,
        source: PathBuf,
        target: PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mount_point = self
            .mount_points
            .get(label)
            .ok_or("Invalid label")?
            .target_path();

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
    pub fn unmount(self) -> Result<(), Box<dyn std::error::Error>> {
        for mount in self.mount_points.values() {
            mount.unmount(UnmountFlags::DETACH)?;
        }

        self.loop_device.detach()?;

        self.mount_dir.close()?;

        Ok(())
    }
}
