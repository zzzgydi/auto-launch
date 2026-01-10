use crate::{AutoLaunch, Error, MacOSLaunchMode, Result};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// macOS implement
impl AutoLaunch {
    /// Create a new AutoLaunch instance
    /// - `app_name`: application name
    /// - `app_path`: application path
    /// - `launch_mode`: launch mode (Launch Agent or AppleScript)
    /// - `args`: startup args passed to the binary
    /// - `bundle_identifiers`: bundle identifiers
    /// - `agent_extra_config`: extra config for Launch Agent
    ///
    /// ## Notes
    ///
    /// The parameters of `AutoLaunch::new` are different on each platform.
    ///
    /// The `app_name` should be same as the basename of the `app_path`
    ///     when using AppleScript mode, or it will be corrected automatically.
    ///
    /// The `app_path` should be the **absolute path** and **exists**,
    ///     otherwise it will cause an error when `enable`.
    ///
    /// In case using AppleScript,
    ///     only `"--hidden"` and `"--minimized"` in `args` are valid.
    pub fn new(
        app_name: &str,
        app_path: &str,
        launch_mode: MacOSLaunchMode,
        args: &[impl AsRef<str>],
        bundle_identifiers: &[impl AsRef<str>],
        agent_extra_config: &str,
    ) -> AutoLaunch {
        let mut name = app_name;
        if launch_mode == MacOSLaunchMode::AppleScript {
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

        AutoLaunch {
            app_name: name.into(),
            app_path: app_path.into(),
            launch_mode,
            args: args.iter().map(|s| s.as_ref().to_string()).collect(),
            bundle_identifiers: bundle_identifiers
                .iter()
                .map(|s| s.as_ref().to_string())
                .collect(),
            agent_extra_config: agent_extra_config.into(),
        }
    }

    /// Enable the AutoLaunch setting
    ///
    /// ## Errors
    ///
    /// - `app_path` does not exist
    /// - `app_path` is not absolute
    ///
    /// #### Launch Agent
    ///
    /// - failed to create dir `~/Library/LaunchAgents`
    /// - failed to create file `~/Library/LaunchAgents/{app_name}.plist`
    /// - failed to write bytes to the file
    ///
    /// #### AppleScript
    ///
    /// - failed to execute the `osascript` command, check the exit status or stderr for details
    pub fn enable(&self) -> Result<()> {
        let path = Path::new(&self.app_path);

        if !path.exists() {
            return Err(Error::AppPathDoesntExist(path.to_path_buf()));
        }

        if !path.is_absolute() {
            return Err(Error::AppPathIsNotAbsolute(path.to_path_buf()));
        }

        match self.launch_mode {
            MacOSLaunchMode::LaunchAgent => self.enable_launch_agent(),
            MacOSLaunchMode::AppleScript => self.enable_applescript(),
        }
    }

    /// Enable using Launch Agent
    fn enable_launch_agent(&self) -> Result<()> {
        let dir = get_dir();
        if !dir.exists() {
            fs::create_dir(&dir)?;
        }

        let mut args = vec![self.app_path.clone()];
        args.extend_from_slice(&self.args);

        let section = args
            .iter()
            .map(|x| format!("<string>{}</string>", x))
            .collect::<String>();

        let identifiers = self
            .bundle_identifiers
            .iter()
            .map(|x| format!("<string>{}</string>", x))
            .collect::<String>();

        let extra_config = if !self.agent_extra_config.is_empty() {
            format!("{}\n  ", self.agent_extra_config)
        } else {
            "".to_string()
        };

        let data = format!(
            "{}\n{}\n\
        <plist version=\"1.0\">\n  \
        <dict>\n  \
            <key>Label</key>\n  \
            <string>{}</string>\n  \
            <key>AssociatedBundleIdentifiers</key>\n  \
            <array>{}</array>\n  \
            <key>ProgramArguments</key>\n  \
            <array>{}</array>\n  \
            <key>RunAtLoad</key>\n  \
            <true/>\n  \
            {}\
        </dict>\n\
        </plist>",
            r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            r#"<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#,
            self.app_name,
            identifiers,
            section,
            extra_config
        );
        let _ = fs::File::create(self.get_file())?.write(data.as_bytes())?;
        Ok(())
    }

    /// Enable using AppleScript
    fn enable_applescript(&self) -> Result<()> {
        let hidden = self
            .args
            .iter()
            .find(|arg| *arg == "--hidden" || *arg == "--minimized");

        let props = format!(
            "{{name:\"{}\",path:\"{}\",hidden:{}}}",
            self.app_name,
            self.app_path,
            hidden.is_some()
        );
        let command = format!("make login item at end with properties {}", props);
        let output = exec_apple_script(&command)?;
        if !output.status.success() {
            return Err(Error::AppleScriptFailed(output.status.code().unwrap_or(1)));
        }
        Ok(())
    }

    /// Disable the AutoLaunch setting
    ///
    /// ## Errors
    ///
    /// #### Launch Agent
    ///
    /// - failed to remove file `~/Library/LaunchAgents/{app_name}.plist`
    ///
    /// #### AppleScript
    ///
    /// - failed to execute the `osascript` command, check the exit status or stderr for details
    pub fn disable(&self) -> Result<()> {
        match self.launch_mode {
            MacOSLaunchMode::LaunchAgent => self.disable_launch_agent(),
            MacOSLaunchMode::AppleScript => self.disable_applescript(),
        }
    }

    /// Disable Launch Agent
    fn disable_launch_agent(&self) -> Result<()> {
        let file = self.get_file();
        if file.exists() {
            fs::remove_file(file)?;
        }
        Ok(())
    }

    /// Disable AppleScript login item
    fn disable_applescript(&self) -> Result<()> {
        let command = format!("delete login item \"{}\"", self.app_name);
        let output = exec_apple_script(&command)?;
        if !output.status.success() {
            return Err(Error::AppleScriptFailed(output.status.code().unwrap_or(1)));
        }
        Ok(())
    }

    /// Check whether the AutoLaunch setting is enabled
    pub fn is_enabled(&self) -> Result<bool> {
        match self.launch_mode {
            MacOSLaunchMode::LaunchAgent => Ok(self.get_file().exists()),
            MacOSLaunchMode::AppleScript => self.is_applescript_enabled(),
        }
    }

    /// Check if AppleScript login item is enabled
    fn is_applescript_enabled(&self) -> Result<bool> {
        let command = "get the name of every login item";
        let output = exec_apple_script(command)?;
        let enable = if output.status.success() {
            let stdout = std::str::from_utf8(&output.stdout).unwrap_or("");
            stdout
                .split(',')
                .map(|x| x.trim())
                .any(|x| x == self.app_name)
        } else {
            false
        };
        Ok(enable)
    }

    /// get the plist file path
    fn get_file(&self) -> PathBuf {
        get_dir().join(format!("{}.plist", self.app_name))
    }
}

/// Get the Launch Agent Dir
fn get_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap()
        .join("Library")
        .join("LaunchAgents")
}

/// Execute the specific AppleScript
fn exec_apple_script(cmd_suffix: &str) -> Result<Output> {
    let command = format!("tell application \"System Events\" to {}", cmd_suffix);
    let output = Command::new("osascript")
        .args(vec!["-e", &command])
        .output()?;
    Ok(output)
}
