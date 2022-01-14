use crate::AutoLaunch;
use std::fs;
use std::io::{Error, Result, Write};
use std::path::PathBuf;
use std::process::{Command, Output};

impl AutoLaunch<'_> {
    pub fn new<'a>(
        app_name: &'a str,
        app_path: &'a str,
        use_launch_agent: bool,
        hidden: bool,
    ) -> AutoLaunch<'a> {
        AutoLaunch::<'a> {
            app_name,
            app_path,
            use_launch_agent,
            hidden,
        }
    }

    pub fn enable(&self) -> Result<()> {
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
                "{{path:\"{}\", hidden:{}, name:\"{}\"}}",
                self.app_path, self.hidden, self.app_name
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
            let command = format!("delete login item {}", self.app_name);
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
                let mut stdout = std::str::from_utf8(&output.stdout)
                    .unwrap_or("")
                    .split(", ");
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

#[test]
fn test_macos_osascript() {
    let app_name = "AutoLaunchTest";
    let app_path = "/Applications/Calculator.app";

    // default test
    let auto_launch = AutoLaunch::new(app_name, app_path, false, false);

    assert_eq!(auto_launch.is_enabled().unwrap(), false);
    assert!(auto_launch.enable().is_ok());
    assert_eq!(auto_launch.is_enabled().unwrap(), true);
    assert!(auto_launch.disable().is_ok());
    assert_eq!(auto_launch.is_enabled().unwrap(), false);

    // test hidden
    let auto_launch = AutoLaunch::new(app_name, app_path, false, true);

    assert_eq!(auto_launch.is_enabled().unwrap(), false);
    assert!(auto_launch.enable().is_ok());
    assert_eq!(auto_launch.is_enabled().unwrap(), true);
    assert!(auto_launch.disable().is_ok());
    assert_eq!(auto_launch.is_enabled().unwrap(), false);
}

#[test]
fn test_macos_user_launch() {
    let app_name = "AutoLaunchTest";
    let app_path = "/Applications/Calculator.app";

    // default test
    let auto_launch = AutoLaunch::new(app_name, app_path, true, false);

    assert_eq!(auto_launch.is_enabled().unwrap(), false);
    assert!(auto_launch.enable().is_ok());
    assert_eq!(auto_launch.is_enabled().unwrap(), true);
    assert!(auto_launch.disable().is_ok());
    assert_eq!(auto_launch.is_enabled().unwrap(), false);

    // test hidden
    let auto_launch = AutoLaunch::new(app_name, app_path, true, true);

    assert_eq!(auto_launch.is_enabled().unwrap(), false);
    assert!(auto_launch.enable().is_ok());
    assert_eq!(auto_launch.is_enabled().unwrap(), true);
    assert!(auto_launch.disable().is_ok());
    assert_eq!(auto_launch.is_enabled().unwrap(), false);
}
