use crate::AutoLaunch;
use std::fs;
use std::io::{Result, Write};
use std::path::PathBuf;

impl AutoLaunch<'_> {
    pub fn new<'a>(app_name: &'a str, app_path: &'a str, hidden: bool) -> AutoLaunch<'a> {
        AutoLaunch::<'a> {
            app_name,
            app_path,
            hidden,
        }
    }

    pub fn enable(&self) -> Result<()> {
        let hidden = if self.hidden { " --hidden" } else { "" };
        let data = format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Version=1.0\n\
            Name={}\n\
            Comment={}startup script\n\
            Exec={}{}\n\
            StartupNotify=false\n\
            Terminal=false",
            self.app_name, self.app_name, self.app_path, hidden
        );

        let dir = get_dir();
        if !dir.exists() {
            fs::create_dir(&dir)?;
        }
        fs::File::create(self.get_file())?.write(data.as_bytes())?;
        Ok(())
    }

    pub fn disable(&self) -> Result<()> {
        let file = self.get_file();
        if file.exists() {
            fs::remove_file(file)
        } else {
            Ok(())
        }
    }

    pub fn is_enabled(&self) -> Result<bool> {
        Ok(self.get_file().exists())
    }

    fn get_file(&self) -> PathBuf {
        get_dir().join(format!("{}.desktop", self.app_name))
    }
}

fn get_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(".config").join("autostart")
}
