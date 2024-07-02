use std::path::PathBuf;

use crate::mount::MountedBakerImage;

pub enum RunEnvironment {
    Chroot,
    SystemdNspawn,
    SystemdVmspawn(PathBuf),
}

impl RunEnvironment {
    pub fn run(
        &self,
        mount_point: &PathBuf,
        user: &str,
        working_dir: &str,
        command: Vec<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mount_point_str = mount_point
            .to_str()
            .ok_or("Failed to convert path to string")?;

        let command_str = command
            .into_iter()
            .map(|arg| format!("\"{}\"", arg))
            .collect::<Vec<String>>()
            .join(" ");

        match &self {
            RunEnvironment::Chroot => {
                let status = std::process::Command::new("chroot")
                    .arg(mount_point_str)
                    .arg("su")
                    .arg("-")
                    .arg(user)
                    .arg("-c")
                    .arg(format!("cd {} && sh -c \"{}\"", working_dir, command_str))
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()?;

                if !status.success() {
                    return Err("Failed to run command".into());
                }
            }
            RunEnvironment::SystemdNspawn => {
                let status = std::process::Command::new("systemd-nspawn")
                    .arg("-D")
                    .arg(mount_point_str)
                    .arg("-u")
                    .arg(user)
                    .arg("--chdir")
                    .arg(working_dir)
                    .arg("sh")
                    .arg("-c")
                    .arg(command_str)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()?;

                if !status.success() {
                    return Err("Failed to run command".into());
                }
            }
            RunEnvironment::SystemdVmspawn(kernel_path) => {
                let status = std::process::Command::new("systemd-vmspawn")
                    .arg("-D")
                    .arg(mount_point_str)
                    .arg("-u")
                    .arg(user)
                    .arg("--chdir")
                    .arg(working_dir)
                    .arg("--linux")
                    .arg(kernel_path.as_os_str())
                    .arg("sh")
                    .arg("-c")
                    .arg(command_str)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()?;

                if !status.success() {
                    return Err("Failed to run command".into());
                }
            }
        }

        Ok(())
    }
}

impl MountedBakerImage {
    pub fn run(
        &self,
        label: &str,
        environment: RunEnvironment,
        user: &str,
        working_dir: &str,
        command: Vec<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mount_point = self.get_mount_point(label)?;

        environment.run(&mount_point, user, working_dir, command)?;

        Ok(())
    }
}
