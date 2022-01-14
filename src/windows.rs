use crate::AutoLaunch;
use std::io::Result;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

static AL_REGKEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run";

impl AutoLaunch<'_> {
    pub fn new<'a>(app_name: &'a str, app_path: &'a str) -> AutoLaunch<'a> {
        AutoLaunch::<'a> { app_name, app_path }
    }

    pub fn enable(&self) -> Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey_with_flags(AL_REGKEY, KEY_SET_VALUE)?
            .set_value::<_, _>(self.app_name, &self.app_path)
    }

    pub fn disable(&self) -> Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        hkcu.open_subkey_with_flags(AL_REGKEY, KEY_SET_VALUE)?
            .delete_value(self.app_name)
    }

    pub fn is_enabled(&self) -> Result<bool> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        Ok(hkcu
            .open_subkey_with_flags(AL_REGKEY, KEY_READ)?
            .get_value::<String, _>(self.app_name)
            .is_ok())
    }
}

#[test]
fn test_win() {
    let app_name = "AutoLaunchTest";
    let app_path = r"C:\Program Files\clash-verge\clash-verge.exe";

    let auto_launch = AutoLaunch::new(app_name, app_path);

    assert_eq!(auto_launch.is_enabled().unwrap(), false);
    assert!(auto_launch.enable().is_ok());
    assert_eq!(auto_launch.is_enabled().unwrap(), true);
    assert!(auto_launch.disable().is_ok());
    assert_eq!(auto_launch.is_enabled().unwrap(), false);
}
