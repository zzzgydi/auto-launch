use crate::{AutoLaunch, Result, WindowsEnableMode};
use windows_registry::{Key, CURRENT_USER, LOCAL_MACHINE};
use windows_result::HRESULT;

const ADMIN_AL_REGKEY: &str = "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Run";
const AL_REGKEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";
const ADMIN_TASK_MANAGER_OVERRIDE_REGKEY: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run32";
const TASK_MANAGER_OVERRIDE_REGKEY: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";
const TASK_MANAGER_OVERRIDE_ENABLED_VALUE: [u8; 12] = [
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const E_ACCESSDENIED: HRESULT = HRESULT::from_win32(0x80070005_u32);
const E_FILENOTFOUND: HRESULT = HRESULT::from_win32(0x80070002_u32);

/// Windows implement
impl AutoLaunch {
    /// Create a new AutoLaunch instance
    /// - `app_name`: application name
    /// - `app_path`: application path
    /// - `enable_mode`: behavior of the enable feature
    /// - `args`: startup args passed to the binary
    ///
    /// ## Notes
    ///
    /// The parameters of `AutoLaunch::new` are different on each platform.
    pub fn new(
        app_name: &str,
        app_path: &str,
        enable_mode: WindowsEnableMode,
        args: &[impl AsRef<str>],
    ) -> AutoLaunch {
        AutoLaunch {
            app_name: app_name.into(),
            app_path: app_path.into(),
            enable_mode,
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
        match self.enable_mode {
            WindowsEnableMode::Dynamic => self
                .enable_as_admin()
                .or_else(|e| {
                    if e.code() == E_ACCESSDENIED {
                        self.enable_as_current_user()
                    } else {
                        Err(e)
                    }
                })
                .map_err(std::io::Error::from)?,
            WindowsEnableMode::CurrentUser => self
                .enable_as_current_user()
                .map_err(std::io::Error::from)?,
            WindowsEnableMode::System => self.enable_as_admin().map_err(std::io::Error::from)?,
        }
        Ok(())
    }

    fn enable_as_admin(&self) -> windows_registry::Result<()> {
        LOCAL_MACHINE.create(ADMIN_AL_REGKEY)?.set_string(
            &self.app_name,
            &format!("{} {}", &self.app_path, &self.args.join(" ")),
        )?;
        // this key maybe not found
        if let Ok(key) = LOCAL_MACHINE.create(ADMIN_TASK_MANAGER_OVERRIDE_REGKEY) {
            key.set_bytes(
                &self.app_name,
                windows_registry::Type::Bytes,
                &TASK_MANAGER_OVERRIDE_ENABLED_VALUE,
            )?;
        }
        Ok(())
    }

    fn enable_as_current_user(&self) -> windows_registry::Result<()> {
        CURRENT_USER.create(AL_REGKEY)?.set_string(
            &self.app_name,
            &format!("{} {}", &self.app_path, &self.args.join(" ")),
        )?;
        // this key maybe not found
        if let Ok(key) = CURRENT_USER.create(TASK_MANAGER_OVERRIDE_REGKEY) {
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
        // try to delete both admin and current user registry values
        match self.disable_as_admin() {
            Ok(()) => {
                // try to delete for current user aswell, ignoring errors
                // this is useful in case the app was previously registered as a system-wide auto launch
                // but changed to a current user only mode
                let _ = self.disable_as_current_user();
                Ok(())
            }
            Err(_e) => {
                self.disable_as_current_user()
                    .map_err(std::io::Error::from)?;
                Ok(())
            }
        }
    }

    fn disable_as_admin(&self) -> windows_registry::Result<()> {
        LOCAL_MACHINE
            .create(ADMIN_AL_REGKEY)?
            .remove_value(&self.app_name)?;
        Ok(())
    }

    fn disable_as_current_user(&self) -> windows_registry::Result<()> {
        CURRENT_USER
            .create(AL_REGKEY)?
            .remove_value(&self.app_name)?;
        Ok(())
    }

    /// Check whether the AutoLaunch setting is enabled
    pub fn is_enabled(&self) -> Result<bool> {
        match self.enable_mode {
            WindowsEnableMode::Dynamic => match self.is_enabled_as_admin() {
                Ok(false) => self.is_enabled_as_current_user(),
                Err(e) if e.code() == E_ACCESSDENIED => self.is_enabled_as_current_user(),
                Ok(enabled) => Ok(enabled),
                Err(e) => Err(e),
            },
            WindowsEnableMode::CurrentUser => self.is_enabled_as_current_user(),
            WindowsEnableMode::System => self.is_enabled_as_admin(),
        }
        .map_err(std::io::Error::from)
        .map_err(Into::into)
    }

    fn is_enabled_as_admin(&self) -> windows_registry::Result<bool> {
        let adm_enabled = LOCAL_MACHINE
            .open(ADMIN_AL_REGKEY)?
            .get_string(&self.app_name)
            .map(|_| true)
            .or_else(|e| {
                if e.code() == E_FILENOTFOUND {
                    Ok(false)
                } else {
                    Err(e)
                }
            })?;
        let task_manager_enabled = self
            .task_manager_enabled(LOCAL_MACHINE, ADMIN_TASK_MANAGER_OVERRIDE_REGKEY)
            .unwrap_or(true);
        Ok(adm_enabled && task_manager_enabled)
    }

    fn is_enabled_as_current_user(&self) -> windows_registry::Result<bool> {
        let al_enabled = CURRENT_USER
            .open(AL_REGKEY)?
            .get_string(&self.app_name)
            .map(|_| true)
            .or_else(|e| {
                if e.code() == E_FILENOTFOUND {
                    Ok(false)
                } else {
                    Err(e)
                }
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
