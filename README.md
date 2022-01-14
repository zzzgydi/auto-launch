# Auto Launch

Auto launch any application or executable at startup. Supports Windows, Mac (via AppleScript or Launch Agent), and Linux.

## Usage

Tha API looks roughly like this:

```rust
impl AutoLaunch {
  fn new(...) -> AutoLaunch {...}

  fn enable(&self) -> io::Result<()> {...}

  fn disable(&self) -> io::Result<()> {...}

  fn is_enabled(&self) -> io::Result<()> {...}
}
```

## Todos

- [ ] test on Linux
- [ ] test on Macos
- [ ] test on Windows

## License

MIT License. See the [License](./LICENSE) file for details.

## Acknowledgement

The project is based on [node-auto-launch](https://github.com/Teamwork/node-auto-launch).
