use crate::AutoLaunch;
use std::fs;
use std::io::{Error, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

impl AutoLaunch<'_> {
    pub fn new<'a>(
        app_name: &'a str,
        app_path: &'a str,
        use_launch_agent: bool,
        hidden: bool,
    ) -> AutoLaunch<'a> {
        let mut name = app_name;
        if !use_launch_agent {
            // the app_name should be same as the executable's name
            // when using login item
            let end = if app_path.ends_with(".app") { 4 } else { 0 };
            let end = app_path.len() - end;
            let begin = match app_path.rfind('/') {
                Some(i) => i + 1,
                None => 0,
            };
            name = &app_path[begin..end];
        }

        AutoLaunch::<'a> {
            app_name: name,
            app_path,
            use_launch_agent,
            hidden,
        }
    }

    pub fn enable(&self) -> Result<()> {
        let path = Path::new(self.app_path);
        if !path.exists() || !path.is_absolute() {
            return Err(Error::from(ErrorKind::InvalidInput));
        }

        if self.use_launch_agent {
            let dir = get_dir();
            if !dir.exists() {
                fs::create_dir(&dir)?;
            }

            let mut args = vec![self.app_path];

            if self.hidden {
                args.push("--hidden");
            }

            let section = args
                .iter()
                .map(|x| format!("<string>{}</string>", x))
                .collect::<String>();

            let data = format!(
                "{}\n{}\n\
            <plist version=\"1.0\">\n  \
            <dict>\n  \
                <key>Label</key>\n  \
                <string>{}</string>\n  \
                <key>ProgramArguments</key>\n  \
                <array>{}</array>\n  \
                <key>RunAtLoad</key>\n  \
                <true/>\n  \
            </dict>\n\
            </plist>",
                r#"<?xml version="1.0" encoding="UTF-8"?>"#,
                r#"<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#,
                self.app_name,
                section
            );
            fs::File::create(self.get_file())?.write(data.as_bytes())?;
            Ok(())
        } else {
            let props = format!(
                "{{name:\"{}\",path:\"{}\",hidden:{}}}",
                self.app_name, self.app_path, self.hidden
            );
            let command = format!("make login item at end with properties {}", props);
            let output = exec_apple_script(&command)?;
            if output.status.success() {
                Ok(())
            } else {
                Err(Error::from_raw_os_error(output.status.code().unwrap_or(1)))
            }
        }
    }

    pub fn disable(&self) -> Result<()> {
        if self.use_launch_agent {
            let file = self.get_file();
            if file.exists() {
                fs::remove_file(file)
            } else {
                Ok(())
            }
        } else {
            let command = format!("delete login item \"{}\"", self.app_name);
            let output = exec_apple_script(&command)?;
            if output.status.success() {
                Ok(())
            } else {
                Err(Error::from_raw_os_error(output.status.code().unwrap_or(1)))
            }
        }
    }

    pub fn is_enabled(&self) -> Result<bool> {
        if self.use_launch_agent {
            Ok(self.get_file().exists())
        } else {
            let command = "get the name of every login item";
            let output = exec_apple_script(command)?;
            let mut enable = false;
            if output.status.success() {
                let stdout = std::str::from_utf8(&output.stdout).unwrap_or("");
                let mut stdout = stdout.split(",").map(|x| x.trim());
                enable = stdout.find(|x| x == &self.app_name).is_some();
            }
            Ok(enable)
        }
    }

    fn get_file(&self) -> PathBuf {
        get_dir().join(format!("{}.plist", self.app_name))
    }
}

fn get_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .join("Library")
        .join("LaunchAgents")
}

fn exec_apple_script(cmd_suffix: &str) -> Result<Output> {
    let command = format!("tell application \"System Events\" to {}", cmd_suffix);
    Command::new("osascript")
        .args(vec!["-e", &command])
        .output()
}
