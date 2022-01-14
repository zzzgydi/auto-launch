//! Auto launch any application or executable at startup. Supports Windows, Mac (via AppleScript or Launch Agent), and Linux.
//!
//! ## Usage
//!
//! The parameters of `AutoLaunch::new` are different on each os.
//! See the function definition or the demo below for details.
//!
//!
//! ### Linux
//!
//! On Linux, it supports `hidden` parameter which means that hidden the app on launch.
//!
//! ```rust
//! # #[cfg(target_os = "linux")]
//! # mod linux {
//! use auto_launch::AutoLaunch;
//!
//! fn main() {
//!     let app_name = "the-app";
//!     let app_path = "/path/to/the-app";
//!     let auto = AutoLaunch::new(app_name, app_path, false);
//!
//!     // enable the auto launch
//!     auto.enable().is_ok();
//!     auto.is_enabled().unwrap();
//!
//!     // disable the auto launch
//!     auto.disable().is_ok();
//!     auto.is_enabled().unwrap();
//! }
//! # }
//! ```
//!
//! ### Macos
//!
//! Macos supports two ways to achieve auto launch (via AppleScript or Launch Agent).
//! When the `use_launch_agent` is true, it will achieve by Launch Agent, otherwise by AppleScript.
//! On Macos, it supports `hidden` parameter which means that hidden the app on launch.
//!
//! **Note**:
//! - The `app_path` should be a absolute path and exists. Otherwise, it will cause an error when `enable`.
//! - When in the AppleScript way, the `app_name` should be same as the basename of `app_path`, or it will be corrected automately.
//!
//! ```rust
//! # #[cfg(target_os = "macos")]
//! # mod macos {
//! use auto_launch::AutoLaunch;
//!
//! fn main() {
//!     let app_name = "the-app";
//!     let app_path = "/path/to/the-app.app";
//!     let auto = AutoLaunch::new(app_name, app_path, false, false);
//!     
//!     // enable the auto launch
//!     auto.enable().is_ok();
//!     auto.is_enabled().unwrap();
//!
//!     // disable the auto launch
//!     auto.disable().is_ok();
//!     auto.is_enabled().unwrap();
//! }
//! # }
//! ```
//!
//! ### Windows
//!
//! On Windows, it will add a registry entry under `\HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`.
//!
//! ```rust
//! # #[cfg(target_os = "windows")]
//! # mod win {
//! use auto_launch::AutoLaunch;
//!
//! fn main() {
//!     let app_name = "the-app";
//!     let app_path = "C:\\path\\to\\the-app.exe";
//!     let auto = AutoLaunch::new(app_name, app_path);
//!
//!     // enable the auto launch
//!     auto.enable().is_ok();
//!     auto.is_enabled().unwrap();
//!
//!     // disable the auto launch
//!     auto.disable().is_ok();
//!     auto.is_enabled().unwrap();
//! }
//! # }
//! ```
//!

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoLaunch<'a> {
    /// The application name
    pub(crate) app_name: &'a str,

    /// The application executable path (absolute path will be better)
    pub(crate) app_path: &'a str,

    #[cfg(target_os = "macos")]
    /// Whether use Launch Agent for implement or use AppleScript
    pub(crate) use_launch_agent: bool,

    #[cfg(not(target_os = "windows"))]
    /// Supports hidden the application on launch
    pub(crate) hidden: bool,
}

impl AutoLaunch<'_> {
    /// get the application name
    pub fn get_app_name(&self) -> &str {
        self.app_name
    }

    /// get the application path
    pub fn get_app_path(&self) -> &str {
        self.app_path
    }

    #[cfg(not(target_os = "windows"))]
    /// get whether it is hidden
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }
}
