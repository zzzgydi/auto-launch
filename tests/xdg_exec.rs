#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn main() {
    use auto_launch::AutoLaunchBuilder;
    use std::{
        env, fs,
        path::PathBuf,
        process::Command,
        thread,
        time::{Duration, Instant},
    };

    let mut args = env::args().skip(1);
    if args.next().as_deref() == Some("--record-args") {
        let output = args.next().unwrap();
        let pending = format!("{output}.pending");
        fs::write(&pending, args.collect::<Vec<_>>().join("\0")).unwrap();
        fs::rename(pending, output).unwrap();
        return;
    }

    // This separate test process owns HOME before starting any threads or child
    // processes. A desktop launch must never touch the developer's login items.
    struct TestHome(PathBuf);
    impl Drop for TestHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let home = env::temp_dir().join(format!("auto-launch-xdg-{}", std::process::id()));
    fs::create_dir(&home).unwrap();
    let home = TestHome(home);
    env::set_var("HOME", &home.0);
    env::set_var("XDG_CONFIG_HOME", home.0.join(".config"));

    let app_dir = home.0.join("Program Files");
    fs::create_dir(&app_dir).unwrap();
    let executable = app_dir.join("Autostart 'quoted' $test` \\ probe.bin");
    fs::copy(env::current_exe().unwrap(), &executable).unwrap();
    let expected = [
        "--profile=work",
        "two words",
        "",
        "a\"b\\c$d`e",
        "single'quote",
        "line\nbreak\tand\rcarriage",
        "%f %F %u %U %i %c %k %%",
        "~ > < | & ; * ? # ( )",
        "雪",
    ];
    let output = home.0.join("received arguments");
    let mut arguments = vec!["--record-args", output.to_str().unwrap()];
    arguments.extend(expected);
    let auto = AutoLaunchBuilder::new()
        .set_app_name("XDG Exec Test")
        .set_app_path(executable.to_str().unwrap())
        .set_args(&arguments)
        .build()
        .unwrap();
    auto.enable().unwrap();
    assert!(auto.is_enabled().unwrap());

    let desktop = home.0.join(".config/autostart/XDG Exec Test.desktop");
    let mut launcher = Command::new("gio")
        .arg("launch")
        .arg(&desktop)
        .spawn()
        .expect("the XDG launch test requires the GLib gio command");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = launcher.try_wait().unwrap() {
            assert!(status.success(), "gio could not launch the desktop entry");
            break;
        }
        if Instant::now() >= deadline {
            let _ = launcher.kill();
            let _ = launcher.wait();
            panic!("gio launch timed out");
        }
        thread::sleep(Duration::from_millis(10));
    }
    while !output.exists() {
        assert!(
            Instant::now() < deadline,
            "desktop entry did not run the executable"
        );
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read(&output).unwrap(), expected.join("\0").as_bytes());
    auto.disable().unwrap();
    assert!(!auto.is_enabled().unwrap());
    assert!(!desktop.exists());
    println!("XDG desktop launch preserved the executable path and every argument");
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
fn main() {}
