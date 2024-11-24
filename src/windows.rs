use crate::{AutoLaunch, Result};
use windows_registry::{Key, CURRENT_USER, LOCAL_MACHINE};

const ADMIN_AL_REGKEY: &str = "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run";
const AL_REGKEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
const ADMIN_TASK_MANAGER_OVERRIDE_REGKEY: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run32";
const TASK_MANAGER_OVERRIDE_REGKEY: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";
const TASK_MANAGER_OVERRIDE_ENABLED_VALUE: [u8; 12] = [
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
        self.enable_as_admin().or_else(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                self.enable_as_current_user().map_err(Into::into)
            }
            _ => Err(e.into()),
        })
    }

    fn enable_as_admin(&self) -> std::io::Result<()> {
        LOCAL_MACHINE.open(ADMIN_AL_REGKEY)?.set_string(
            &self.app_name,
            &format!("{} {}", &self.app_path, &self.args.join(" ")),
        )?;
        // this key maybe not found
        if let Ok(key) = LOCAL_MACHINE.open(ADMIN_TASK_MANAGER_OVERRIDE_REGKEY) {
            key.set_bytes(
                &self.app_name,
                windows_registry::Type::Bytes,
                &TASK_MANAGER_OVERRIDE_ENABLED_VALUE,
            )?;
        }
        Ok(())
    }

    fn enable_as_current_user(&self) -> std::io::Result<()> {
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
            )?;
        // this key maybe not found
        if let Ok(key) = CURRENT_USER.open(TASK_MANAGER_OVERRIDE_REGKEY) {
            key.set_bytes(
                &self.app_name,
                windows_registry::Type::Bytes,
                &TASK_MANAGER_OVERRIDE_ENABLED_VALUE,
            )?;
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
        self.disable_as_admin().or_else(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                self.disable_as_current_user().map_err(Into::into)
            }
            _ => Err(e.into()),
        })
    }

    fn disable_as_admin(&self) -> std::io::Result<()> {
        LOCAL_MACHINE
            .open(ADMIN_AL_REGKEY)?
            .remove_value(&self.app_name)?;
        Ok(())
    }

    fn disable_as_current_user(&self) -> std::io::Result<()> {
        CURRENT_USER.open(AL_REGKEY)?.remove_value(&self.app_name)?;
        Ok(())
    }

    /// Check whether the AutoLaunch setting is enabled
    pub fn is_enabled(&self) -> Result<bool> {
        match self.is_enabled_as_admin() {
            Ok(false) => self.is_enabled_as_current_user().map_err(Into::into),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                self.is_enabled_as_current_user().map_err(Into::into)
            }
            Ok(enabled) => Ok(enabled),
            Err(e) => Err(e.into()),
        }
    }

    fn is_enabled_as_admin(&self) -> std::io::Result<bool> {
        let adm_enabled = LOCAL_MACHINE
            .open(ADMIN_AL_REGKEY)?
            .get_string(&self.app_name)
            .map(|_| true)
            .map_err(std::io::Error::from)
            .or_else(|e| match e.kind() {
                std::io::ErrorKind::NotFound => Ok(false),
                _ => Err(e),
            })?;
        let task_manager_enabled = self
            .task_manager_enabled(LOCAL_MACHINE, ADMIN_TASK_MANAGER_OVERRIDE_REGKEY)
            .unwrap_or(true);
        Ok(adm_enabled && task_manager_enabled)
    }

    fn is_enabled_as_current_user(&self) -> std::io::Result<bool> {
        let al_enabled = CURRENT_USER
            .open(AL_REGKEY)?
            .get_string(&self.app_name)
            .map(|_| true)
            .map_err(std::io::Error::from)
            .or_else(|e| match e.kind() {
                std::io::ErrorKind::NotFound => Ok(false),
                _ => Err(e),
            })?;
        let task_manager_enabled = self
            .task_manager_enabled(CURRENT_USER, TASK_MANAGER_OVERRIDE_REGKEY)
            .unwrap_or(true);
        Ok(al_enabled && task_manager_enabled)
    }

    fn task_manager_enabled(&self, hk: &Key, path: &str) -> Option<bool> {
        let task_manager_override_raw_value = hk.open(path).ok()?.get_value(&self.app_name).ok()?;
        last_eight_bytes_all_zeros(&task_manager_override_raw_value)
    }
}

fn last_eight_bytes_all_zeros(bytes: &[u8]) -> Option<bool> {
    if bytes.len() < 8 {
        return None;
    }
    Some(bytes.iter().rev().take(8).all(|v| *v == 0u8))
}
