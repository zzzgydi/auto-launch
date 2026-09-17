use crate::{AutoLaunch, Error, MacOSLaunchMode, Result};
use smappservice_rs::{AppService, ServiceStatus, ServiceType};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
};

// User-supplied values are passed in argv, never interpolated into source.
const APPLESCRIPT_ENABLE: &str = r#"tell application "System Events" to make login item at end with properties {name:(item 1 of argv), path:(item 2 of argv), hidden:((item 3 of argv) is "true")}"#;
const APPLESCRIPT_DISABLE: &str =
    r#"tell application "System Events" to delete login item (item 1 of argv)"#;
const APPLESCRIPT_IS_ENABLED: &str =
    r#"tell application "System Events" to exists login item (item 1 of argv)"#;

/// macOS implement
impl AutoLaunch {
    /// Create a new AutoLaunch instance
    /// - `app_name`: application name
    /// - `app_path`: application path
    /// - `launch_mode`: launch mode (Launch Agent, AppleScript, or SMAppService)
    /// - `args`: startup args passed to the binary
    /// - `bundle_identifiers`: bundle identifiers
    /// - `agent_extra_config`: raw XML entries appended to the Launch Agent dictionary
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
    /// In Launch Agent mode, pass plain strings; XML escaping is handled internally.
    /// `agent_extra_config` remains a raw XML fragment.
    ///
    /// In case using AppleScript,
    ///     only `"--hidden"` and `"--minimized"` in `args` are valid.
    ///
    /// In case using SMAppService (macOS 13+), `app_name` and `app_path` can be empty strings
    ///     as it registers the running application.
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
    /// #### SMAppService
    ///
    /// - failed to register app with SMAppService API (macOS 13+)
    pub fn enable(&self) -> Result<()> {
        if self.launch_mode == MacOSLaunchMode::SMAppService {
            let app_service = AppService::new(ServiceType::MainApp);
            match app_service.register() {
                Ok(()) => return Ok(()),
                Err(e) => return Err(Error::SMAppServiceRegistrationFailed(e.code())),
            }
        }

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
            MacOSLaunchMode::SMAppService => unreachable!("SMAppService mode handled above"),
        }
    }

    /// Enable using Launch Agent
    fn enable_launch_agent(&self) -> Result<()> {
        let dir = get_dir()?;
        if !dir.exists() {
            fs::create_dir(&dir)?;
        }

        let data = build_launch_agent_plist(
            &self.app_name,
            &self.app_path,
            &self.args,
            &self.bundle_identifiers,
            &self.agent_extra_config,
        );
        let _ = fs::File::create(self.get_file()?)?.write(data.as_bytes())?;
        Ok(())
    }

    /// Enable using AppleScript
    fn enable_applescript(&self) -> Result<()> {
        let hidden = self
            .args
            .iter()
            .find(|arg| *arg == "--hidden" || *arg == "--minimized");

        let output = exec_apple_script(
            APPLESCRIPT_ENABLE,
            &[
                &self.app_name,
                &self.app_path,
                if hidden.is_some() { "true" } else { "false" },
            ],
        )?;
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
    /// #### SMAppService
    ///
    /// - failed to unregister app with SMAppService API (macOS 13+)
    pub fn disable(&self) -> Result<()> {
        match self.launch_mode {
            MacOSLaunchMode::LaunchAgent => self.disable_launch_agent(),
            MacOSLaunchMode::AppleScript => self.disable_applescript(),
            MacOSLaunchMode::SMAppService => self.disable_smappservice(),
        }
    }

    /// Disable SMAppService
    fn disable_smappservice(&self) -> Result<()> {
        let app_service = AppService::new(ServiceType::MainApp);
        match app_service.unregister() {
            Ok(()) => Ok(()),
            Err(e) => Err(Error::SMAppServiceUnregistrationFailed(e.code())),
        }
    }

    /// Disable Launch Agent
    fn disable_launch_agent(&self) -> Result<()> {
        let file = self.get_file()?;
        if file.exists() {
            fs::remove_file(file)?;
        }
        Ok(())
    }

    /// Disable AppleScript login item
    fn disable_applescript(&self) -> Result<()> {
        let output = exec_apple_script(APPLESCRIPT_DISABLE, &[&self.app_name])?;
        if !output.status.success() {
            return Err(Error::AppleScriptFailed(output.status.code().unwrap_or(1)));
        }

        Ok(())
    }

    /// Check whether the AutoLaunch setting is enabled
    ///
    /// #### SMAppService
    ///
    /// - Check if the app is registered with SMAppService
    pub fn is_enabled(&self) -> Result<bool> {
        match self.launch_mode {
            MacOSLaunchMode::LaunchAgent => Ok(self.get_file()?.exists()),
            MacOSLaunchMode::AppleScript => self.is_applescript_enabled(),
            MacOSLaunchMode::SMAppService => self.is_smappservice_enabled(),
        }
    }

    /// Check if SMAppService is enabled
    fn is_smappservice_enabled(&self) -> Result<bool> {
        let app_service = AppService::new(ServiceType::MainApp);
        Ok(app_service.status() == ServiceStatus::Enabled)
    }

    /// Check if AppleScript login item is enabled
    fn is_applescript_enabled(&self) -> Result<bool> {
        let output = exec_apple_script(APPLESCRIPT_IS_ENABLED, &[&self.app_name])?;
        Ok(output.status.success()
            && std::str::from_utf8(&output.stdout).unwrap_or("").trim() == "true")
    }

    /// get the plist file path
    fn get_file(&self) -> Result<PathBuf> {
        Ok(get_dir()?.join(format!("{}.plist", self.app_name)))
    }
}

/// Get the Launch Agent Dir
fn get_dir() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Failed to find home directory")
    })?;
    Ok(home_dir.join("Library").join("LaunchAgents"))
}

/// Execute the specific AppleScript
fn exec_apple_script(script: &str, args: &[&str]) -> Result<Output> {
    let command = format!("on run argv\n{script}\nend run");
    let output = Command::new("osascript")
        .args(["-e", &command, "--"])
        .args(args)
        .output()?;
    Ok(output)
}

fn build_launch_agent_plist(
    app_name: &str,
    app_path: &str,
    args: &[String],
    bundle_identifiers: &[String],
    agent_extra_config: &str,
) -> String {
    let mut full_args = vec![app_path.to_string()];
    full_args.extend_from_slice(args);

    let section = full_args
        .iter()
        .map(|x| format!("<string>{}</string>", escape_plist_text(x)))
        .collect::<String>();

    let identifiers = bundle_identifiers
        .iter()
        .map(|x| format!("<string>{}</string>", escape_plist_text(x)))
        .collect::<String>();

    let extra_config = if !agent_extra_config.is_empty() {
        format!("{}\n  ", agent_extra_config)
    } else {
        String::new()
    };

    format!(
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
        escape_plist_text(app_name),
        identifiers,
        section,
        extra_config
    )
}

// These values are XML element text, not attributes or raw plist fragments.
fn escape_plist_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn test_applescript_preserves_arguments() {
        // Numeric character IDs avoid osascript's text/list output formatting.
        // This runs the real transport without addressing System Events.
        let values = [
            "",
            "Plain App",
            "My \"App\"",
            r"App\new",
            r"App\test",
            r"trailing\",
            "a, b",
            " leading and trailing ",
            "--leading",
            "雪😀",
            "line\nbreak\tand\rcarriage",
            "\" & (error \"injected\") & \"",
        ];
        for value in values {
            let output = exec_apple_script("return id of (item 1 of argv)", &[value]).unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let stdout = String::from_utf8(output.stdout).unwrap();
            let actual: Vec<u32> = stdout
                .trim()
                .split(',')
                .filter(|part| !part.trim().is_empty())
                .map(|part| part.trim().parse().unwrap())
                .collect();
            let expected: Vec<u32> = value.chars().map(u32::from).collect();
            assert_eq!(actual, expected, "{value:?}");
        }
    }

    #[test]
    fn test_applescript_login_properties() {
        // Evaluate the production property record using the application's
        // terminology only. No command is sent to System Events.
        let properties = APPLESCRIPT_ENABLE.split_once("with properties ").unwrap().1;
        let script = format!(
            "using terms from application \"System Events\"\n\
             set props to {properties}\n\
             return {{(id of (name of props)) is (id of (item 1 of argv)), \
             (id of (path of props)) is (id of (item 2 of argv)), hidden of props}}\n\
             end using terms from"
        );
        for hidden in ["true", "false"] {
            let output = exec_apple_script(
                &script,
                &["My \"App\", 雪", r"/Applications/A\new.app", hidden],
            )
            .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8(output.stdout).unwrap().trim(),
                format!("true, true, {hidden}")
            );
        }
    }

    #[test]
    fn test_applescript_login_commands_compile() {
        // Compile the exact production commands but never execute their bodies:
        // do not create, delete, or query the developer's actual login items.
        for script in [
            APPLESCRIPT_ENABLE,
            APPLESCRIPT_DISABLE,
            APPLESCRIPT_IS_ENABLED,
        ] {
            let guarded = format!("if false then\n{script}\nend if\nreturn true");
            let output = exec_apple_script(
                &guarded,
                &["My \"App\"", r"/Applications/A\new.app", "true"],
            )
            .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "true");
        }
    }

    // Use Apple's parser to verify decoded values, not just serialized text.
    fn plist_value(data: &str, key: &str, kind: &str) -> String {
        let mut parser = Command::new("/usr/bin/plutil")
            .args([
                "-extract", key, "raw", "-expect", kind, "-n", "-o", "-", "--", "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        parser
            .stdin
            .take()
            .unwrap()
            .write_all(data.as_bytes())
            .unwrap();
        let output = parser.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "plutil failed for {key}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        String::from_utf8(output.stdout).unwrap()
    }

    #[test]
    fn test_launch_agent_preserves_literal_text() {
        let name = "Test & <App> \"雪\"";
        let path = "/Applications/A&B <雪>.app/Contents/MacOS/App";
        let args: Vec<String> = [
            "",
            "two words",
            "\"double\" and 'single'",
            "A&B",
            "A<B",
            "A>B",
            "&amp;",
            "]]>",
            "</string><string>injected",
            "雪😀",
            "\\",
            "line\nbreak\ttab",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let identifiers = vec!["com.example.app".into(), "literal&<value>".into()];
        let data = build_launch_agent_plist(name, path, &args, &identifiers, "");

        assert_eq!(plist_value(&data, "Label", "string"), name);
        assert_eq!(
            plist_value(&data, "ProgramArguments", "array"),
            (args.len() + 1).to_string()
        );
        assert_eq!(plist_value(&data, "ProgramArguments.0", "string"), path);
        for (index, expected) in args.iter().enumerate() {
            assert_eq!(
                plist_value(&data, &format!("ProgramArguments.{}", index + 1), "string"),
                *expected
            );
        }
        assert_eq!(
            plist_value(&data, "AssociatedBundleIdentifiers", "array"),
            identifiers.len().to_string()
        );
        for (index, expected) in identifiers.iter().enumerate() {
            assert_eq!(
                plist_value(
                    &data,
                    &format!("AssociatedBundleIdentifiers.{index}"),
                    "string"
                ),
                *expected
            );
        }
        assert_eq!(plist_value(&data, "RunAtLoad", "bool"), "true");
    }

    #[test]
    fn test_launch_agent_keeps_extra_config_as_xml() {
        let extra = "<key>KeepAlive</key><true/><key>EnvironmentVariables</key><dict><key>VALUE</key><string>A&amp;B</string></dict>";
        let data = build_launch_agent_plist("Test", "/Applications/Test.app", &[], &[], extra);
        assert_eq!(plist_value(&data, "KeepAlive", "bool"), "true");
        assert_eq!(
            plist_value(&data, "EnvironmentVariables.VALUE", "string"),
            "A&B"
        );
    }

    #[test]
    fn test_build_launch_agent_plist() {
        let data = build_launch_agent_plist(
            "TestApp",
            "/Applications/TestApp.app",
            &vec!["--flag".into()],
            &vec!["com.example.testapp".into()],
            "<key>KeepAlive</key><true/>",
        );

        assert!(data.contains("<key>Label</key>"));
        assert!(data.contains("<string>TestApp</string>"));
        assert!(data.contains("<key>AssociatedBundleIdentifiers</key>"));
        assert!(data.contains("<string>com.example.testapp</string>"));
        assert!(data.contains("<key>ProgramArguments</key>"));
        assert!(data.contains("<string>/Applications/TestApp.app</string>"));
        assert!(data.contains("<string>--flag</string>"));
        assert!(data.contains("<key>RunAtLoad</key>"));
        assert!(data.contains("<key>KeepAlive</key><true/>"));
    }
}
