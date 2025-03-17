use crate::{AutoLaunch, Result, WindowsEnableMode};
use std::io;
use windows_registry::{Key, CURRENT_USER, LOCAL_MACHINE};
use windows_result::HRESULT;

const AL_REGKEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
const TASK_MANAGER_OVERRIDE_REGKEY: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
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
        self.enable_with_root_key(LOCAL_MACHINE)
    }

    fn enable_as_current_user(&self) -> windows_registry::Result<()> {
        self.enable_with_root_key(CURRENT_USER)
    }

    fn enable_with_root_key(&self, root_key: &Key) -> windows_registry::Result<()> {
        root_key.create(AL_REGKEY)?.set_string(
            &self.app_name,
            format!("{} {}", &self.app_path, &self.args.join(" ")),
        )?;

        match root_key
            .options()
            .write()
            .open(TASK_MANAGER_OVERRIDE_REGKEY)
        {
            Ok(key) => key.set_bytes(
                &self.app_name,
                windows_registry::Type::Bytes,
                &TASK_MANAGER_OVERRIDE_ENABLED_VALUE,
            )?,
            Err(error) if error.code() == E_FILENOTFOUND => {
                return Ok(());
            }
            Err(error) => {
                return Err(error);
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
        // try to delete both admin and current user registry values
        if let Err(error) = self.disable_as_admin() {
            if error.code() == E_ACCESSDENIED {
                match self.enable_mode {
                    // Fail if our enable mode is system but we don't have the access
                    WindowsEnableMode::System => return Err(std::io::Error::from(error).into()),
                    // Otherwise ignore this error
                    _ => {}
                }
            } else {
                return Err(std::io::Error::from(error).into());
            }
        }
        self.disable_as_current_user()
            .map_err(std::io::Error::from)?;
        Ok(())
    }

    fn disable_as_admin(&self) -> windows_registry::Result<()> {
        self.disable_with_root_key(LOCAL_MACHINE)
    }

    fn disable_as_current_user(&self) -> windows_registry::Result<()> {
        self.disable_with_root_key(CURRENT_USER)
    }

    fn disable_with_root_key(&self, root_key: &Key) -> windows_registry::Result<()> {
        match root_key
            .options()
            .write()
            .open(AL_REGKEY)
            .and_then(|key| key.remove_value(&self.app_name))
        {
            Ok(_) => Ok(()),
            Err(error) if error.code() == E_FILENOTFOUND => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// Check whether the AutoLaunch setting is enabled
    pub fn is_enabled(&self) -> Result<bool> {
        let is_registered =
            self.is_registered(LOCAL_MACHINE)? || self.is_registered(CURRENT_USER)?;
        if !is_registered {
            return Ok(false);
        }
        let is_task_manager_enabled = self.is_task_manager_enabled(LOCAL_MACHINE)?
            && self.is_task_manager_enabled(CURRENT_USER)?;
        Ok(is_task_manager_enabled)
    }

    fn is_registered(&self, root_key: &Key) -> io::Result<bool> {
        let registered = match root_key
            .open(AL_REGKEY)
            .and_then(|key| key.get_string(&self.app_name))
        {
            Ok(_) => true,
            Err(error) if error.code() == E_FILENOTFOUND => false,
            Err(error) => {
                return Err(error.into());
            }
        };
        Ok(registered)
    }

    fn is_task_manager_enabled(&self, root_key: &Key) -> io::Result<bool> {
        let task_manager_enabled = match root_key
            .open(TASK_MANAGER_OVERRIDE_REGKEY)
            .and_then(|key| key.get_value(&self.app_name))
        {
            Ok(value) => last_eight_bytes_all_zeros(&value).unwrap_or(true),
            Err(error) if error.code() == E_FILENOTFOUND => true,
            Err(error) => {
                return Err(error.into());
            }
        };
        Ok(task_manager_enabled)
    }
}

fn last_eight_bytes_all_zeros(bytes: &[u8]) -> std::result::Result<bool, &str> {
    if bytes.len() < 8 {
        Err("Bytes too short")
    } else {
        Ok(bytes.iter().rev().take(8).all(|v| *v == 0u8))
    }
}
