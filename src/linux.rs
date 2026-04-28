use crate::{AutoLaunch, LinuxLaunchMode, Result};
use std::{fs, io::Write, path::PathBuf};

/// Linux implement
impl AutoLaunch {
    /// Create a new AutoLaunch instance
    /// - `app_name`: application name
    /// - `app_path`: application path
    /// - `launch_mode`: launch mode (XDG Autostart or systemd)
    /// - `args`: startup args passed to the binary
    ///
    /// ## Notes
    ///
    /// The parameters of `AutoLaunch::new` are different on each platform.
    pub fn new(
        app_name: &str,
        app_path: &str,
        launch_mode: LinuxLaunchMode,
        args: &[impl AsRef<str>],
    ) -> AutoLaunch {
        AutoLaunch {
            app_name: app_name.into(),
            app_path: app_path.into(),
            launch_mode,
            args: args.iter().map(|s| s.as_ref().to_string()).collect(),
        }
    }

    /// Enable the AutoLaunch setting
    ///
    /// ## Errors
    ///
    /// - failed to create directory
    /// - failed to create file
    /// - failed to write bytes to the file
    /// - failed to enable systemd service (if using systemd mode)
    pub fn enable(&self) -> Result<()> {
        match self.launch_mode {
            LinuxLaunchMode::XdgAutostart => self.enable_xdg_autostart(),
            LinuxLaunchMode::SystemdUser | LinuxLaunchMode::SystemdSystem => self.enable_systemd(),
        }
    }

    /// Enable using XDG Autostart (.desktop file)
    fn enable_xdg_autostart(&self) -> Result<()> {
        let data = build_xdg_autostart_data(&self.app_name, &self.app_path, &self.args);

        let dir = get_xdg_autostart_dir()?;
        if !dir.exists() {
            fs::create_dir_all(&dir).or_else(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(e)
                }
            })?;
        }
        let file_path = self.get_xdg_desktop_file()?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_path)?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    /// Enable using systemd service
    fn enable_systemd(&self) -> Result<()> {
        // Create systemd service file content
        let data = build_systemd_service_data(&self.app_name, &self.app_path, &self.args);

        // Create systemd directory
        let dir = get_systemd_dir(self.launch_mode)?;
        if !dir.exists() {
            fs::create_dir_all(&dir).or_else(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(e)
                }
            })?;
        }

        // Write service file
        let service_file = self.get_systemd_service_file()?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&service_file)?;
        file.write_all(data.as_bytes())?;

        // Enable and start the service using systemctl
        self.systemctl_enable()?;

        Ok(())
    }

    /// Run systemctl enable command.
    fn systemctl_enable(&self) -> Result<()> {
        let service_name = format!("{}.service", self.app_name);
        let args: &[&str] = match self.launch_mode {
            LinuxLaunchMode::SystemdUser => &["--user", "enable", &service_name],
            LinuxLaunchMode::SystemdSystem => &["enable", &service_name],
            LinuxLaunchMode::XdgAutostart => unreachable!("XDG mode does not use systemctl"),
        };
        let output = std::process::Command::new("systemctl")
            .args(args)
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Failed to enable systemd service: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            )
            .into());
        }

        Ok(())
    }

    /// Disable the AutoLaunch setting
    ///
    /// ## Errors
    ///
    /// - failed to remove file
    /// - failed to disable systemd service (if using systemd mode)
    pub fn disable(&self) -> Result<()> {
        match self.launch_mode {
            LinuxLaunchMode::XdgAutostart => self.disable_xdg_autostart(),
            LinuxLaunchMode::SystemdUser | LinuxLaunchMode::SystemdSystem => self.disable_systemd(),
        }
    }

    /// Disable XDG Autostart
    fn disable_xdg_autostart(&self) -> Result<()> {
        let file = self.get_xdg_desktop_file()?;
        if file.exists() {
            fs::remove_file(file)?;
        }
        Ok(())
    }

    /// Disable systemd service
    fn disable_systemd(&self) -> Result<()> {
        // Disable the service
        self.systemctl_disable()?;

        // Remove service file
        let service_file = self.get_systemd_service_file()?;
        if service_file.exists() {
            fs::remove_file(service_file)?;
        }

        // Reload systemd daemon
        let daemon_reload_args: &[&str] = match self.launch_mode {
            LinuxLaunchMode::SystemdUser => &["--user", "daemon-reload"],
            LinuxLaunchMode::SystemdSystem => &["daemon-reload"],
            LinuxLaunchMode::XdgAutostart => unreachable!("XDG mode does not use systemctl"),
        };
        let _ = std::process::Command::new("systemctl")
            .args(daemon_reload_args)
            .output();

        Ok(())
    }

    /// Run systemctl disable command.
    fn systemctl_disable(&self) -> Result<()> {
        let service_name = format!("{}.service", self.app_name);
        let args: &[&str] = match self.launch_mode {
            LinuxLaunchMode::SystemdUser => &["--user", "disable", &service_name],
            LinuxLaunchMode::SystemdSystem => &["disable", &service_name],
            LinuxLaunchMode::XdgAutostart => unreachable!("XDG mode does not use systemctl"),
        };
        let output = std::process::Command::new("systemctl")
            .args(args)
            .output()?;

        // Don't fail if the service is not enabled
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("No such file or directory") && !stderr.contains("not loaded") {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to disable systemd service: {}", stderr),
                )
                .into());
            }
        }

        Ok(())
    }

    /// Check whether the AutoLaunch setting is enabled
    pub fn is_enabled(&self) -> Result<bool> {
        match self.launch_mode {
            LinuxLaunchMode::XdgAutostart => Ok(self.get_xdg_desktop_file()?.exists()),
            LinuxLaunchMode::SystemdUser | LinuxLaunchMode::SystemdSystem => {
                self.is_systemd_enabled()
            }
        }
    }

    /// Check if systemd service is enabled
    fn is_systemd_enabled(&self) -> Result<bool> {
        let service_name = format!("{}.service", self.app_name);
        let args: &[&str] = match self.launch_mode {
            LinuxLaunchMode::SystemdUser => &["--user", "is-enabled", &service_name],
            LinuxLaunchMode::SystemdSystem => &["is-enabled", &service_name],
            LinuxLaunchMode::XdgAutostart => unreachable!("XDG mode does not use systemctl"),
        };
        let output = std::process::Command::new("systemctl")
            .args(args)
            .output()?;

        // systemctl is-enabled returns:
        // - "enabled" with exit code 0 if enabled
        // - "disabled" with exit code 1 if disabled
        // - other states or errors with other exit codes
        Ok(output.status.success())
    }

    /// Get the XDG desktop entry file path
    fn get_xdg_desktop_file(&self) -> Result<PathBuf> {
        Ok(get_xdg_autostart_dir()?.join(format!("{}.desktop", self.app_name)))
    }

    /// Get the systemd service file path
    fn get_systemd_service_file(&self) -> Result<PathBuf> {
        Ok(get_systemd_dir(self.launch_mode)?.join(format!("{}.service", self.app_name)))
    }
}

fn build_xdg_autostart_data(app_name: &str, app_path: &str, args: &[String]) -> String {
    format!(
        "[Desktop Entry]\n\
        Type=Application\n\
        Version=1.0\n\
        Name={}\n\
        Comment={} startup script\n\
        Exec={} {}\n\
        StartupNotify=false\n\
        Terminal=false",
        app_name,
        app_name,
        app_path,
        args.join(" ")
    )
}

fn build_systemd_service_data(app_name: &str, app_path: &str, args: &[String]) -> String {
    let args_str = if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    };

    format!(
        "[Unit]\n\
        Description={}\n\
        After=default.target\n\
        \n\
        [Service]\n\
        Type=simple\n\
        ExecStart={}{}\n\
        Restart=on-failure\n\
        RestartSec=10\n\
        \n\
        [Install]\n\
        WantedBy=default.target",
        app_name, app_path, args_str
    )
}

/// Get the XDG autostart directory
fn get_xdg_autostart_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Failed to find home directory",
        )
    })?;
    Ok(home_dir.join(".config").join("autostart"))
}

/// Get the systemd service directory.
fn get_systemd_dir(mode: LinuxLaunchMode) -> Result<PathBuf> {
    match mode {
        LinuxLaunchMode::SystemdSystem => Ok(PathBuf::from("/etc/systemd/system")),
        LinuxLaunchMode::SystemdUser => {
            let home_dir = dirs::home_dir().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Failed to find home directory",
                )
            })?;
            Ok(home_dir.join(".config").join("systemd").join("user"))
        }
        LinuxLaunchMode::XdgAutostart => unreachable!("XDG mode does not use systemd dir"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_xdg_autostart_data() {
        let data = build_xdg_autostart_data(
            "TestApp",
            "/opt/test-app",
            &vec!["--flag".into(), "value".into()],
        );

        assert!(data.contains("Type=Application"));
        assert!(data.contains("Name=TestApp"));
        assert!(data.contains("Comment=TestApp startup script"));
        assert!(data.contains("Exec=/opt/test-app --flag value"));
        assert!(data.contains("StartupNotify=false"));
        assert!(data.contains("Terminal=false"));
    }

    #[test]
    fn test_build_systemd_service_data() {
        let data = build_systemd_service_data("TestApp", "/opt/test-app", &vec!["--flag".into()]);

        assert!(data.contains("Description=TestApp"));
        assert!(data.contains("After=default.target"));
        assert!(data.contains("ExecStart=/opt/test-app --flag"));
        assert!(data.contains("Restart=on-failure"));
        assert!(data.contains("WantedBy=default.target"));
    }
}
