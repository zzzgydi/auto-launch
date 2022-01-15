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

Or you can construct the AutoLaunch by using `AutoLaunchBuilder`.

### Linux

On Linux, it supports `hidden` parameter which means that hidden the app on launch.

```rust
use auto_launch::AutoLaunch;

fn main() {
    let app_name = "the-app";
    let app_path = "/path/to/the-app";
    let auto = AutoLaunch::new(app_name, app_path, false);

    // enable the auto launch
    auto.enable().is_ok();
    auto.is_enabled().unwrap();

    // disable the auto launch
    auto.disable().is_ok();
    auto.is_enabled().unwrap();
}
```

### Macos

Macos supports two ways to achieve auto launch (via AppleScript or Launch Agent).
When the `use_launch_agent` is true, it will achieve by Launch Agent, otherwise by AppleScript.
On Macos, it supports `hidden` parameter which means that hidden the app on launch.

**Note**:

- The `app_path` should be a absolute path and exists. Otherwise, it will cause an error when `enable`.
- When in the AppleScript way, the `app_name` should be same as the basename of `app_path`, or it will be corrected automately.

```rust
use auto_launch::AutoLaunch;

fn main() {
    let app_name = "the-app";
    let app_path = "/path/to/the-app.app";
    let auto = AutoLaunch::new(app_name, app_path, false, false);

    // enable the auto launch
    auto.enable().is_ok();
    auto.is_enabled().unwrap();

    // disable the auto launch
    auto.disable().is_ok();
    auto.is_enabled().unwrap();
}
```

### Windows

On Windows, it will add a registry entry under `\HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`.

```rust
use auto_launch::AutoLaunch;

fn main() {
    let app_name = "the-app";
    let app_path = "C:\\path\\to\\the-app.exe";
    let auto = AutoLaunch::new(app_name, app_path);

    // enable the auto launch
    auto.enable().is_ok();
    auto.is_enabled().unwrap();

    // disable the auto launch
    auto.disable().is_ok();
    auto.is_enabled().unwrap();
}
```

### Builder

AutoLaunch Builder helps to eliminate the constructor difference on various platforms.

```rust
use auto_launch::*;

fn main() -> std::io::Result<()> {
    let auto = AutoLaunchBuilder::new()
        .set_app_name("the-app")
        .set_app_path("/path/to/the-app")
        .set_use_launch_agent(true)
        .set_hidden(true)
        .build();

    auto.enable()?;
    auto.is_enabled()?;

    auto.disable()?;
    auto.is_enabled()?;

    Ok(())
}
```

## License

MIT License. See the [License](./LICENSE) file for details.

## Acknowledgement

The project is based on [node-auto-launch](https://github.com/Teamwork/node-auto-launch).
