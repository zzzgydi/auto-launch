use crate::{AutoLaunch, Result};
use windows_registry::{Key, CURRENT_USER, LOCAL_MACHINE};

static ADMIN_AL_REGKEY: &str = "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run";
static AL_REGKEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
static ADMIN_TASK_MANAGER_OVERRIDE_REGKEY: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run32";
static TASK_MANAGER_OVERRIDE_REGKEY: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";
static TASK_MANAGER_OVERRIDE_ENABLED_VALUE: [u8; 12] = [
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Windows implement
impl AutoLaunch {
    /// Create a new AutoLaunch instance
    /// - `app_name`: application name
    /// - `app_path`: application path
    /// - `args`: startup args passed to the binary
    ///
    /// ## Notes
    ///
    /// The parameters of `AutoLaunch::new` are different on each platform.
    pub fn new(app_name: &str, app_path: &str, args: &[impl AsRef<str>]) -> AutoLaunch {
        AutoLaunch {
            app_name: app_name.into(),
            app_path: app_path.into(),
            args: args.iter().map(|s| s.as_ref().to_string()).collect(),
        }
    }

    /// Enable the AutoLaunch setting
    ///
    /// ## Errors
    ///
    /// - failed to open the registry key
    /// - failed to set value
    pub fn enable(&self) -> Result<()> {
        if let Ok(key) = LOCAL_MACHINE.open(ADMIN_AL_REGKEY) {
            key.set_string(
                &self.app_name,
                &format!("{} {}", &self.app_path, &self.args.join(" ")),
            )
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("failed to set {ADMIN_AL_REGKEY}: {}", e),
                )
            })?;
            // this key maybe not found
            if let Ok(key) = LOCAL_MACHINE.open(ADMIN_TASK_MANAGER_OVERRIDE_REGKEY) {
                key.set_bytes(&self.app_name, &TASK_MANAGER_OVERRIDE_ENABLED_VALUE)
                    .map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("failed to set {ADMIN_TASK_MANAGER_OVERRIDE_REGKEY}: {}", e),
                        )
                    })?;
            }
        } else {
            CURRENT_USER
                .open(AL_REGKEY)
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("failed to open {AL_REGKEY}: {}", e),
                    )
                })?
                .set_string(
                    &self.app_name,
                    &format!("{} {}", &self.app_path, &self.args.join(" ")),
                )
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("failed to set {AL_REGKEY}: {}", e),
                    )
                })?;
            // this key maybe not found
            if let Ok(key) = CURRENT_USER.open(TASK_MANAGER_OVERRIDE_REGKEY) {
                key.set_bytes(&self.app_name, &TASK_MANAGER_OVERRIDE_ENABLED_VALUE)
                    .map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("failed to set {TASK_MANAGER_OVERRIDE_REGKEY}: {}", e),
                        )
                    })?;
            }
        }

        Ok(())
    }

    /// Disable the AutoLaunch setting
    ///
    /// ## Errors
    ///
    /// - failed to open the registry key
    /// - failed to delete value
    pub fn disable(&self) -> Result<()> {
        if let Ok(reg) = LOCAL_MACHINE.open(ADMIN_AL_REGKEY) {
            reg.remove_value(&self.app_name).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("failed to remove {ADMIN_AL_REGKEY}: {}", e),
                )
            })?;
        } else {
            CURRENT_USER
                .open(AL_REGKEY)
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("failed to open {AL_REGKEY}: {}", e),
                    )
                })?
                .remove_value(&self.app_name)
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("failed to remove {AL_REGKEY}: {}", e),
                    )
                })?;
        }
        Ok(())
    }

    /// Check whether the AutoLaunch setting is enabled
    pub fn is_enabled(&self) -> Result<bool> {
        // check if the app is enabled in the admin registry
        // use `KEY_ALL_ACCESS` to ensure have admin permission
        if let Ok(key) = LOCAL_MACHINE.open(ADMIN_AL_REGKEY) {
            let adm_enabled = key.get_string(&self.app_name).is_ok();
            let task_manager_enabled =
                self.task_manager_enabled(LOCAL_MACHINE, ADMIN_TASK_MANAGER_OVERRIDE_REGKEY);
            Ok(adm_enabled && task_manager_enabled.unwrap_or(true))
        } else {
            let al_enabled = CURRENT_USER
                .open(AL_REGKEY)
                .map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("failed to open {AL_REGKEY}: {}", e),
                    )
                })?
                .get_string(&self.app_name)
                .is_ok();
            let task_manager_enabled =
                self.task_manager_enabled(CURRENT_USER, TASK_MANAGER_OVERRIDE_REGKEY);

            Ok(al_enabled && task_manager_enabled.unwrap_or(true))
        }
    }

    fn task_manager_enabled(&self, hk: &Key, path: &str) -> Option<bool> {
        let task_manager_override_raw_value = hk.open(path).ok()?.get_bytes(&self.app_name).ok()?;
        last_eight_bytes_all_zeros(&task_manager_override_raw_value)
    }
}

fn last_eight_bytes_all_zeros(bytes: &[u8]) -> Option<bool> {
    if bytes.len() < 8 {
        return None;
    }
    Some(bytes.iter().rev().take(8).all(|v| *v == 0u8))
}
