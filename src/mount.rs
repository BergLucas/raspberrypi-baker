use std::{fs, thread::sleep, time::Duration};

use glob::glob;
use loopdev::{LoopControl, LoopDevice};
use sys_mount::{Mount, Unmount, UnmountFlags};
use tempdir::TempDir;
use udev::Device;

use crate::images::BakerImage;

pub struct MountedBakerImage {
    loop_device: LoopDevice,
    mount_points: Vec<Mount>,
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
                    .ok_or("Failed to get device label")?;

                let mount_point = mount_dir.path().join(label);

                fs::create_dir_all(mount_point.as_path())?;

                Ok(Mount::new(partition_device, mount_point)?)
            })
            .collect::<Result<Vec<Mount>, Box<dyn std::error::Error>>>()?;

        Ok(MountedBakerImage {
            loop_device,
            mount_points,
        })
    }
    pub fn unmount(self) -> Result<(), Box<dyn std::error::Error>> {
        for mount in self.mount_points {
            mount.unmount(UnmountFlags::DETACH)?;
        }

        self.loop_device.detach()?;

        Ok(())
    }
}
