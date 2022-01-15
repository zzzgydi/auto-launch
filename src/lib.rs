//! Auto launch any application or executable at startup. Supports Windows, Mac (via AppleScript or Launch Agent), and Linux.
//!
//! ## Usage
//!
//! The parameters of `AutoLaunch::new` are different on each platform.
//! See the function definition or the demo below for details.
//!
//! Or you can construct the AutoLaunch by using `AutoLaunchBuilder`.
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
//! ### Builder
//!
//! AutoLaunch Builder helps to eliminate the constructor difference
//! on various platforms.
//!
//! ```rust
//! use auto_launch::*;
//!
//! fn main() -> std::io::Result<()> {
//!     let auto = AutoLaunchBuilder::new()
//!         .set_app_name("the-app")
//!         .set_app_path("/path/to/the-app")
//!         .set_use_launch_agent(true)
//!         .set_hidden(true)
//!         .build();
//!     
//!     auto.enable()?;
//!     auto.is_enabled()?;
//!     
//!     auto.disable()?;
//!     auto.is_enabled()?;
//!
//!     Ok(())
//! }
//! ```
//!

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// The parameters of `AutoLaunch::new` are different on each platform.
///
/// ### Linux
///
/// ```rust
/// # #[cfg(target_os = "linux")]
/// # {
/// # use auto_launch::AutoLaunch;
/// # let app_name = "the-app";
/// # let app_path = "/path/to/the-app";
/// # let hidden = false;
/// AutoLaunch::new(app_name, app_path, hidden);
/// # }
/// ```
///
/// ### Macos
///
/// ```rust
/// # #[cfg(target_os = "macos")]
/// # {
/// # use auto_launch::AutoLaunch;
/// # let app_name = "the-app";
/// # let app_path = "/path/to/the-app";
/// # let use_launch_agent = false;
/// # let hidden = false;
/// AutoLaunch::new(app_name, app_path, use_launch_agent, hidden);
/// # }
/// ```
///
/// ### Windows
///
/// ```rust
/// # #[cfg(target_os = "windows")]
/// # {
/// # use auto_launch::AutoLaunch;
/// # let app_name = "the-app";
/// # let app_path = "/path/to/the-app";
/// AutoLaunch::new(app_name, app_path);
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoLaunch {
    /// The application name
    pub(crate) app_name: String,

    /// The application executable path (absolute path will be better)
    pub(crate) app_path: String,

    #[cfg(target_os = "macos")]
    /// Whether use Launch Agent for implement or use AppleScript
    pub(crate) use_launch_agent: bool,

    #[cfg(not(target_os = "windows"))]
    /// Supports hidden the application on launch
    pub(crate) hidden: bool,
}

impl AutoLaunch {
    /// check whether it is support the platform
    ///
    /// ## Usage
    ///
    /// ```rust
    /// use auto_launch::AutoLaunch;
    ///
    /// dbg!(AutoLaunch::is_support());
    /// ```
    pub fn is_support() -> bool {
        cfg!(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows",
        ))
    }

    /// get the application name
    pub fn get_app_name(&self) -> &str {
        &self.app_name
    }

    /// get the application path
    pub fn get_app_path(&self) -> &str {
        &self.app_path
    }

    #[cfg(not(target_os = "windows"))]
    /// get whether it is hidden
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }
}

#[derive(Debug, Default, Clone)]
/// AutoLaunch Builder helps to eliminate the constructor difference
/// on various platforms.
///
/// ## Notes
///
/// The builder will not check whether the app_path matches the platform-specify file path.
///
/// ## Usage
///
/// ```rust
/// use auto_launch::*;
///
/// fn main() -> std::io::Result<()> {
///     let auto = AutoLaunchBuilder::new()
///         .set_app_name("the-app")
///         .set_app_path("/path/to/the-app")
///         .set_use_launch_agent(true)
///         .set_hidden(true)
///         .build();
///     
///     auto.enable()?;
///     auto.is_enabled()?;
///     
///     auto.disable()?;
///     auto.is_enabled()?;
///
///     Ok(())
/// }
/// ```
pub struct AutoLaunchBuilder {
    pub app_name: Option<String>,

    pub app_path: Option<String>,

    pub use_launch_agent: bool,

    pub hidden: bool,
}

impl AutoLaunchBuilder {
    pub fn new() -> AutoLaunchBuilder {
        AutoLaunchBuilder::default()
    }

    /// Set the `app_name`
    pub fn set_app_name(&mut self, name: &str) -> &mut Self {
        self.app_name = Some(name.into());
        self
    }

    /// Set the `app_path`
    pub fn set_app_path(&mut self, path: &str) -> &mut Self {
        self.app_path = Some(path.into());
        self
    }

    /// Set the `use_launch_agent`
    pub fn set_use_launch_agent(&mut self, use_launch_agent: bool) -> &mut Self {
        self.use_launch_agent = use_launch_agent;
        self
    }

    /// Set the `hidden`
    pub fn set_hidden(&mut self, hidden: bool) -> &mut Self {
        self.hidden = hidden;
        self
    }

    /// Construct a AutoLaunch instance
    ///
    /// ## Panics
    ///
    /// - `app_name` is None
    /// - `app_path` is None
    pub fn build(&self) -> AutoLaunch {
        if self.app_name.is_none() {
            panic!("The `app_name` should not be None.");
        }

        if self.app_path.is_none() {
            panic!("The `app_path` should not be None.");
        }

        let app_name = self.app_name.clone().unwrap();
        let app_path = self.app_path.clone().unwrap();

        #[cfg(target_os = "linux")]
        return AutoLaunch::new(&app_name, &app_path, self.hidden);
        #[cfg(target_os = "macos")]
        return AutoLaunch::new(&app_name, &app_path, self.use_launch_agent, self.hidden);
        #[cfg(target_os = "windows")]
        return AutoLaunch::new(&app_name, &app_path);
    }
}
