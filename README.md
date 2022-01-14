# Auto Launch

Auto launch any application or executable at startup. Supports Windows, Mac (via AppleScript or Launch Agent), and Linux.

As for `how does it work`, see [here](https://github.com/Teamwork/node-auto-launch#how-does-it-work) for details.

If you find any bugs, welcome to PR or issue.

## Usage

The parameters of `AutoLaunch::new` are different on each platform.
See the function definition or the demo below for details.

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

## License

MIT License. See the [License](./LICENSE) file for details.

## Acknowledgement

The project is based on [node-auto-launch](https://github.com/Teamwork/node-auto-launch).
