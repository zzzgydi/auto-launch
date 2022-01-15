use crate::AutoLaunch;
use std::io::Result;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

static AL_REGKEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";

/// Windows implement
impl AutoLaunch {
    /// Create a new AutoLaunch instance
    /// - `app_name`: application name
    /// - `app_path`: application path
    ///
    /// ## Notes
    ///
    /// The parameters of `AutoLaunch::new` are different on each platform.
    pub fn new(app_name: &str, app_path: &str) -> AutoLaunch {
        AutoLaunch {
            app_name: app_name.into(),
            app_path: app_path.into(),
        }
    }

    /// Enable the AutoLaunch setting
    ///
    /// ## Errors
    ///
    /// - failed to open the registry key
    /// - failed to set value
    pub fn enable(&self) -> Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey_with_flags(AL_REGKEY, KEY_SET_VALUE)?
            .set_value::<_, _>(&self.app_name, &self.app_path)
    }

    /// Disable the AutoLaunch setting
    ///
    /// ## Errors
    ///
    /// - failed to open the registry key
    /// - failed to delete value
    pub fn disable(&self) -> Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey_with_flags(AL_REGKEY, KEY_SET_VALUE)?
            .delete_value(&self.app_name)
    }

    /// Check whether the AutoLaunch setting is enabled
    pub fn is_enabled(&self) -> Result<bool> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        Ok(hkcu
            .open_subkey_with_flags(AL_REGKEY, KEY_READ)?
            .get_value::<String, _>(&self.app_name)
            .is_ok())
    }
}
