# Auto Launch

[![Crates.io](https://img.shields.io/crates/v/auto-launch)](https://crates.io/crates/auto-launch)
[![API reference](https://img.shields.io/docsrs/auto-launch/latest)](https://docs.rs/auto-launch/)
[![License](https://img.shields.io/crates/l/auto-launch)](./LICENSE)

Auto launch any application or executable at startup. Supports Windows, Mac (via AppleScript or Launch Agent), and Linux.

How does it work? See [Teamwork/node-auto-launch](https://github.com/Teamwork/node-auto-launch#how-does-it-work) for details.

If you find any bugs, welcome to PR or issue.

## Usage

The parameters of `AutoLaunch::new` are different on each platform.
See the function definition or the demo below for details.

`AutoLaunchBuilder` helps to eliminate the constructor difference on various platforms.

```rust
use auto_launch::*;

fn main() {
    let auto = AutoLaunchBuilder::new()
        .set_app_name("the-app")
        .set_app_path("/path/to/the-app")
        .set_macos_launch_mode(MacOSLaunchMode::LaunchAgent)
        .build()
        .unwrap();

    auto.enable().unwrap();
    auto.is_enabled().unwrap();

    auto.disable().unwrap();
    auto.is_enabled().unwrap();
}
```

### Linux

Linux supports two ways to achieve auto launch:
- **XDG Autostart**: Uses `.desktop` files in `~/.config/autostart/` (default)
- **systemd**: Uses systemd user services in `~/.config/systemd/user/`

```rust
use auto_launch::{AutoLaunch, LinuxLaunchMode};

fn main() {
    let app_name = "the-app";
    let app_path = "/path/to/the-app";
    
    // Use XDG Autostart (default method)
    let auto = AutoLaunch::new(app_name, app_path, LinuxLaunchMode::XdgAutostart, &[] as &[&str]);
    
    // Or use systemd user service
    // let auto = AutoLaunch::new(app_name, app_path, LinuxLaunchMode::Systemd, &[] as &[&str]);

    // enable the auto launch
    auto.enable().is_ok();
    auto.is_enabled().unwrap();

    // disable the auto launch
    auto.disable().is_ok();
    auto.is_enabled().unwrap();
}
```

### macOS

macOS supports two ways to achieve auto launch:
- **Launch Agent**: Uses plist files in `~/Library/LaunchAgents/` (default)
- **AppleScript**: Uses AppleScript to add login items

**Note**:

- The `app_path` should be a absolute path and exists. Otherwise, it will cause an error when `enable`.
- In case using AppleScript, the `app_name` should be same as the basename of `app_path`, or it will be corrected automatically.
- In case using AppleScript, only `--hidden` and `--minimized` in `args` are valid, which means that hide the app on launch.

```rust
use auto_launch::{AutoLaunch, MacOSLaunchMode};

fn main() {
    let app_name = "the-app";
    let app_path = "/path/to/the-app.app";
    
    // Use Launch Agent (default method)
    let auto = AutoLaunch::new(app_name, app_path, MacOSLaunchMode::LaunchAgent, &[] as &[&str], &[] as &[&str], "");
    
    // Or use AppleScript
    // let auto = AutoLaunch::new(app_name, app_path, MacOSLaunchMode::AppleScript, &[] as &[&str], &[] as &[&str], "");

    // enable the auto launch
    auto.enable().is_ok();
    auto.is_enabled().unwrap();

    // disable the auto launch
    auto.disable().is_ok();
    auto.is_enabled().unwrap();
}
```

### Windows

On Windows, it will add registry entries under `\HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Run` and `\HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run`.

It will also detect if startup is disabled inside Task Manager or the Windows settings UI, and can re-enable after being disabled in one of those.

```rust
use auto_launch::AutoLaunch;

fn main() {
    let app_name = "the-app";
    let app_path = "C:\\path\\to\\the-app.exe";
    let auto = AutoLaunch::new(app_name, app_path, &[] as &[&str]);

    // enable the auto launch
    auto.enable().is_ok();
    auto.is_enabled().unwrap();

    // disable the auto launch
    auto.disable().is_ok();
    auto.is_enabled().unwrap();
}
```

## License

MIT License. See the [License](./LICENSE) file for details.

## Acknowledgement

The project is based on [node-auto-launch](https://github.com/Teamwork/node-auto-launch).
