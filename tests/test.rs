#[cfg(test)]
mod union_tests {
    use auto_launch::{AutoLaunch, AutoLaunchBuilder};
    use std::env::current_dir;

    fn get_test_bin() -> String {
        let test_bin = if cfg!(target_os = "windows") {
            "auto-launch-test.exe"
        } else {
            "auto-launch-test"
        };
        let test_bin = current_dir()
            .unwrap()
            .join("test-exe/target/release")
            .join(test_bin);

        // if not exists, check the test exe
        assert!(test_bin.exists());
        test_bin.as_os_str().to_string_lossy().into_owned()
    }

    #[test]
    fn test_support() {
        assert!(AutoLaunch::is_support());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_new() {
        let name_1 = "AutoLaunchTest"; // different name
        let name_2 = "auto-launch-test"; // same name
        let app_path = get_test_bin();
        let app_path = app_path.as_str();

        // applescript
        let auto1 = AutoLaunch::new(name_1, app_path, false, false);
        let auto2 = AutoLaunch::new(name_2, app_path, false, false);
        // launch agent
        let auto3 = AutoLaunch::new(name_1, app_path, true, false);
        let auto4 = AutoLaunch::new(name_2, app_path, true, false);

        // app_name will be revised
        assert_eq!(auto1.get_app_name(), name_2);
        assert_eq!(auto2.get_app_name(), name_2);
        // there is not limited when using launch agent
        assert_eq!(auto3.get_app_name(), name_1);
        assert_eq!(auto4.get_app_name(), name_2);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_main() {
        let app_name = "auto-launch-test";
        let app_path = get_test_bin();
        let app_path = app_path.as_str();

        // path not exists
        let app_name_not = "Calculator1";
        let app_path_not = "/Applications/Calculator1.app";

        // use applescript
        let auto1 = AutoLaunch::new(app_name, app_path, false, false);
        assert_eq!(auto1.get_app_name(), app_name);
        assert!(auto1.enable().is_ok());
        assert!(auto1.is_enabled().unwrap());
        assert!(auto1.disable().is_ok());
        assert!(!auto1.is_enabled().unwrap());

        let auto2 = AutoLaunch::new(app_name_not, app_path_not, false, false);
        assert_eq!(auto2.get_app_name(), app_name_not);
        assert!(auto2.enable().is_err());
        assert!(!auto2.is_enabled().unwrap());

        // use launch agent
        let auto1 = AutoLaunch::new(app_name, app_path, true, false);
        assert_eq!(auto1.get_app_name(), app_name);
        assert!(auto1.enable().is_ok());
        assert!(auto1.is_enabled().unwrap());
        assert!(auto1.disable().is_ok());
        assert!(!auto1.is_enabled().unwrap());

        let auto2 = AutoLaunch::new(app_name, app_path_not, true, false);
        assert_eq!(auto2.get_app_name(), app_name); // will not change the name
        assert!(auto2.enable().is_err());
        assert!(!auto2.is_enabled().unwrap());
        assert!(auto2.disable().is_ok());
        assert!(!auto2.is_enabled().unwrap());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux() {
        let app_name = "AutoLaunchTest";
        let app_path = get_test_bin();
        let app_path = app_path.as_str();

        // default test
        let auto1 = AutoLaunch::new(app_name, app_path, false);

        assert_eq!(auto1.get_app_name(), app_name);
        assert!(auto1.enable().is_ok());
        assert!(auto1.is_enabled().unwrap());
        assert!(auto1.disable().is_ok());
        assert!(!auto1.is_enabled().unwrap());

        // test hidden
        let auto2 = AutoLaunch::new(app_name, app_path, true);

        assert_eq!(auto2.get_app_name(), app_name);
        assert!(auto2.enable().is_ok());
        assert!(auto2.is_enabled().unwrap());
        assert!(auto2.disable().is_ok());
        assert!(!auto2.is_enabled().unwrap());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows() {
        let app_name = "AutoLaunchTest";
        let app_path = get_test_bin();
        let app_path = app_path.as_str();

        let auto = AutoLaunch::new(app_name, app_path);

        assert_eq!(auto.get_app_name(), app_name);
        assert!(auto.enable().is_ok());
        assert!(auto.is_enabled().unwrap());
        assert!(auto.disable().is_ok());
        assert!(!auto.is_enabled().unwrap());
    }

    #[test]
    fn test_builder() {
        let app_name = "auto-launch-test";
        let app_path = get_test_bin();
        let app_path = app_path.as_str();

        let auto = AutoLaunchBuilder::new()
            .set_app_name(app_name)
            .set_app_path(app_path)
            .set_hidden(true)
            .build();

        #[cfg(not(target_os = "windows"))]
        assert!(auto.is_hidden());

        assert_eq!(auto.get_app_name(), app_name);
        assert!(auto.enable().is_ok());
        assert!(auto.is_enabled().unwrap());
        assert!(auto.disable().is_ok());
        assert!(!auto.is_enabled().unwrap());
    }
}
