use crate::{AutoLaunch, Result};
use winreg::enums::RegType::REG_BINARY;
use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, KEY_READ, KEY_SET_VALUE,
};
use winreg::{RegKey, RegValue};

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
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(reg) = hklm.open_subkey_with_flags(ADMIN_AL_REGKEY, KEY_SET_VALUE) {
            reg.set_value::<_, _>(
                &self.app_name,
                &format!("{} {}", &self.app_path, &self.args.join(" ")),
            )?;
            // this key maybe not found
            if let Ok(reg) =
                hklm.open_subkey_with_flags(ADMIN_TASK_MANAGER_OVERRIDE_REGKEY, KEY_SET_VALUE)
            {
                reg.set_raw_value(
                    &self.app_name,
                    &RegValue {
                        vtype: REG_BINARY,
                        bytes: TASK_MANAGER_OVERRIDE_ENABLED_VALUE.to_vec(),
                    },
                )?;
            }
        } else {
            hkcu.open_subkey_with_flags(AL_REGKEY, KEY_SET_VALUE)?
                .set_value::<_, _>(
                    &self.app_name,
                    &format!("{} {}", &self.app_path, &self.args.join(" ")),
                )?;
            // this key maybe not found
            if let Ok(reg) =
                hkcu.open_subkey_with_flags(TASK_MANAGER_OVERRIDE_REGKEY, KEY_SET_VALUE)
            {
                reg.set_raw_value(
                    &self.app_name,
                    &RegValue {
                        vtype: REG_BINARY,
                        bytes: TASK_MANAGER_OVERRIDE_ENABLED_VALUE.to_vec(),
                    },
                )?;
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
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

        if let Ok(reg) = hklm.open_subkey_with_flags(ADMIN_AL_REGKEY, KEY_SET_VALUE) {
            reg.delete_value(&self.app_name)?;
        } else {
            hkcu.open_subkey_with_flags(AL_REGKEY, KEY_SET_VALUE)?
                .delete_value(&self.app_name)?;
        }
        Ok(())
    }

    /// Check whether the AutoLaunch setting is enabled
    pub fn is_enabled(&self) -> Result<bool> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        // check if the app is enabled in the admin registry
        // use `KEY_ALL_ACCESS` to ensure have admin permission
        if let Ok(reg) = hklm.open_subkey_with_flags(ADMIN_AL_REGKEY, KEY_ALL_ACCESS) {
            let adm_enabled = reg.get_value::<String, _>(&self.app_name).is_ok();
            let task_manager_enabled =
                self.task_manager_enabled(hklm, ADMIN_TASK_MANAGER_OVERRIDE_REGKEY);
            Ok(adm_enabled && task_manager_enabled.unwrap_or(true))
        } else {
            let al_enabled = hkcu
                .open_subkey_with_flags(AL_REGKEY, KEY_READ)?
                .get_value::<String, _>(&self.app_name)
                .is_ok();
            let task_manager_enabled =
                self.task_manager_enabled(hkcu, TASK_MANAGER_OVERRIDE_REGKEY);

            Ok(al_enabled && task_manager_enabled.unwrap_or(true))
        }
    }

    fn task_manager_enabled(&self, hk: RegKey, path: &str) -> Option<bool> {
        let task_manager_override_raw_value = hk
            .open_subkey_with_flags(path, KEY_READ)
            .ok()?
            .get_raw_value(&self.app_name)
            .ok()?;
        Some(last_eight_bytes_all_zeros(
            &task_manager_override_raw_value.bytes,
        )?)
    }
}

fn last_eight_bytes_all_zeros(bytes: &[u8]) -> Option<bool> {
    if bytes.len() < 8 {
        return None;
    }
    Some(bytes.iter().rev().take(8).all(|v| *v == 0u8))
}
