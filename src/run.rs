use std::{collections::HashMap, path::PathBuf};

use crate::mount::MountedImage;

pub enum RunEnvironment {
    Chroot,
    SystemdNspawn,
    SystemdVmspawn(PathBuf),
}

impl RunEnvironment {
    pub fn run(
        &self,
        mount_point: &PathBuf,
        environment_variables: &HashMap<String, String>,
        user: &str,
        working_dir: &str,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mount_point_str = mount_point
            .to_str()
            .ok_or("Failed to convert path to string")?;

        let mut environment_variables_str = environment_variables
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<String>>()
            .join(" ");

        if !environment_variables_str.is_empty() {
            environment_variables_str.push_str("; ");
        }

        match &self {
            RunEnvironment::Chroot => {
                let status = std::process::Command::new("chroot")
                    .arg(mount_point_str)
                    .arg("su")
                    .arg("-")
                    .arg(user)
                    .arg("-c")
                    .arg(format!(
                        "cd '{}' && sh -c '{}{}'",
                        working_dir, environment_variables_str, command,
                    ))
                    .status()?;

                if !status.success() {
                    return Err("Failed to run command".into());
                }
            }
            RunEnvironment::SystemdNspawn => {
                let status = std::process::Command::new("systemd-nspawn")
                    .arg("-q")
                    .arg("-D")
                    .arg(mount_point_str)
                    .arg("-u")
                    .arg(user)
                    .arg("sh")
                    .arg("-c")
                    .arg(format!(
                        "cd '{}' && sh -c '{}{}'",
                        working_dir, environment_variables_str, command,
                    ))
                    .status()?;

                if !status.success() {
                    return Err("Failed to run command".into());
                }
            }
            RunEnvironment::SystemdVmspawn(kernel_path) => {
                let status = std::process::Command::new("systemd-vmspawn")
                    .arg("-q")
                    .arg("-D")
                    .arg(mount_point_str)
                    .arg("-u")
                    .arg(user)
                    .arg("--linux")
                    .arg(kernel_path.as_os_str())
                    .arg("sh")
                    .arg("-c")
                    .arg(format!(
                        "cd '{}' && sh -c '{}{}'",
                        working_dir, environment_variables_str, command,
                    ))
                    .status()?;

                if !status.success() {
                    return Err("Failed to run command".into());
                }
            }
        }

        Ok(())
    }
}

impl MountedImage {
    pub fn run(
        &self,
        label: &str,
        environment: RunEnvironment,
        environment_variables: &HashMap<String, String>,
        user: &str,
        working_dir: &str,
        command: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mount_point = self.get_mount_point(label)?;

        environment.run(
            &mount_point,
            environment_variables,
            user,
            working_dir,
            command,
        )?;

        Ok(())
    }
}
