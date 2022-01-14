#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Debug)]
pub struct AutoLaunch<'a> {
    pub(crate) app_name: &'a str,

    pub(crate) app_path: &'a str,

    #[cfg(target_os = "macos")]
    pub(crate) use_launch_agent: bool,

    #[cfg(not(target_os = "windows"))]
    pub(crate) hidden: bool,
}

impl AutoLaunch<'_> {
    pub fn get_app_name(&self) -> &str {
        self.app_name
    }

    pub fn get_app_path(&self) -> &str {
        self.app_path
    }

    #[cfg(not(target_os = "windows"))]
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }
}
