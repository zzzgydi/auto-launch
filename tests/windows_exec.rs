#[cfg(windows)]
fn main() {
    use auto_launch::{AutoLaunch, WindowsEnableMode};
    use std::{env, fs, path::PathBuf, time::SystemTime};
    use windows_registry::CURRENT_USER;

    const RUN: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str =
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

    let mut args = env::args().skip(1);
    if args.next().as_deref() == Some("--record-args") {
        let output = args.next().unwrap();
        // Include argv[0] to check which executable actually started.
        let received = (env::args().next().unwrap(), args.collect::<Vec<_>>());
        fs::write(output, format!("{received:?}")).unwrap();
        return;
    }

    struct Fixture {
        dir: PathBuf,
        name: String,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            for path in [RUN, APPROVED] {
                if let Ok(key) = CURRENT_USER.options().write().open(path) {
                    let _ = key.remove_value(&self.name);
                }
            }
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let name = format!("auto-launch-exec-{}-{nonce}", std::process::id());
    let dir = env::temp_dir().join(&name);
    fs::create_dir(&dir).unwrap();
    let fixture = Fixture { dir, name };
    let app_dir = fixture.dir.join("John Doe 雪");
    fs::create_dir(&app_dir).unwrap();
    let executable = app_dir.join("Startup App.exe");
    fs::copy(env::current_exe().unwrap(), &executable).unwrap();

    let expected = [
        "--autostart",
        "two words",
        "",
        "tab\tvalue",
        "line\nbreak",
        "a\"b",
        r#"a\"b"#,
        r#"a\\"b"#,
        r"C:\plain\",
        r"C:\two words\",
        r"C:\two words\\",
        "%PATH% & | < > ^",
        "雪😀",
        "",
    ];
    let output = fixture.dir.join("received arguments");
    let mut arguments = vec!["--record-args", output.to_str().unwrap()];
    arguments.extend(expected);
    let auto = AutoLaunch::new(
        &fixture.name,
        executable.to_str().unwrap(),
        WindowsEnableMode::CurrentUser,
        &arguments,
    );
    auto.enable().unwrap();
    let command = CURRENT_USER
        .open(RUN)
        .unwrap()
        .get_string(&fixture.name)
        .unwrap();
    // Preserve callers that already quoted their executable as a workaround.
    // Identical registry bytes mean both inputs exercise the same launch below.
    let quoted = AutoLaunch::new(
        &fixture.name,
        &format!("\"{}\"", executable.display()),
        WindowsEnableMode::CurrentUser,
        &arguments,
    );
    quoted.enable().unwrap();
    assert_eq!(
        CURRENT_USER
            .open(RUN)
            .unwrap()
            .get_string(&fixture.name)
            .unwrap(),
        command
    );

    // Do not leave an active startup entry while waiting for the child.
    auto.disable().unwrap();
    assert!(CURRENT_USER
        .open(RUN)
        .unwrap()
        .get_string(&fixture.name)
        .is_err());

    launch(&command);
    let expected = (executable.to_str().unwrap(), expected.to_vec());
    assert_eq!(fs::read_to_string(output).unwrap(), format!("{expected:?}"));

    // Invalid input must not overwrite an already registered command.
    let run_key = CURRENT_USER.create(RUN).unwrap();
    run_key.set_string(&fixture.name, &command).unwrap();
    for (path, args) in [("invalid\"path.exe", vec![]), ("app.exe", vec!["bad\0arg"])] {
        let invalid = AutoLaunch::new(&fixture.name, path, WindowsEnableMode::CurrentUser, &args);
        assert!(invalid.enable().is_err());
        assert_eq!(run_key.get_string(&fixture.name).unwrap(), command);
    }
    run_key.remove_value(&fixture.name).unwrap();
    println!("Windows Run command preserved the executable path and every argument");
}

#[cfg(windows)]
fn launch(command: &str) {
    use std::{io, mem::size_of, ptr};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::Threading::{
            CreateProcessW, GetExitCodeProcess, TerminateProcess, WaitForSingleObject,
            PROCESS_INFORMATION, STARTUPINFOW,
        },
    };

    struct Child(PROCESS_INFORMATION, bool);
    impl Drop for Child {
        fn drop(&mut self) {
            // SAFETY: These handles came from a successful CreateProcessW call
            // and remain owned by this guard until they are closed here.
            unsafe {
                if !self.1 {
                    TerminateProcess(self.0.hProcess, 1);
                    WaitForSingleObject(self.0.hProcess, 5_000);
                }
                CloseHandle(self.0.hThread);
                CloseHandle(self.0.hProcess);
            }
        }
    }

    let mut command: Vec<u16> = command.encode_utf16().chain(Some(0)).collect();
    let startup = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut info = PROCESS_INFORMATION::default();
    // SAFETY: The mutable command buffer is NUL-terminated; both structures are
    // initialized and valid for the call. NULL application name intentionally
    // exercises executable selection from the exact registry command line.
    let created = unsafe {
        CreateProcessW(
            ptr::null(),
            command.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            0,
            0,
            ptr::null(),
            ptr::null(),
            &startup,
            &mut info,
        )
    };
    assert_ne!(created, 0, "CreateProcessW: {}", io::Error::last_os_error());
    let mut child = Child(info, false);
    // SAFETY: The process handle is owned by child and is still open.
    let result = unsafe { WaitForSingleObject(child.0.hProcess, 10_000) };
    assert_eq!(result, WAIT_OBJECT_0, "child timed out or wait failed");
    child.1 = true;
    let mut code = 0;
    // SAFETY: code is writable and the process handle remains valid.
    let success = unsafe { GetExitCodeProcess(child.0.hProcess, &mut code) };
    assert_ne!(success, 0, "GetExitCodeProcess failed");
    assert_eq!(code, 0, "argument recorder failed");
}

#[cfg(not(windows))]
fn main() {}
