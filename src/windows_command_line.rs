use std::io;

pub(crate) fn build_command_line(app_path: &str, args: &[String]) -> io::Result<String> {
    // Accept one surrounding quote pair for callers that worked around the
    // old unquoted Run command. Quotes inside a Windows path remain invalid.
    let app_path = app_path
        .strip_prefix('"')
        .and_then(|path| path.strip_suffix('"'))
        .unwrap_or(app_path);
    if app_path.is_empty() || app_path.contains(['\0', '"']) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "app_path must be nonempty and contain no embedded quotes or NUL characters",
        ));
    }
    if args.iter().any(|arg| arg.contains('\0')) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "startup arguments cannot contain NUL characters",
        ));
    }

    let mut command = format!("\"{app_path}\"");
    for arg in args {
        command.push(' ');
        push_argument(&mut command, arg);
    }
    Ok(command)
}

fn push_argument(command: &mut String, arg: &str) {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        command.push_str(arg);
        return;
    }

    // Use the Windows C runtime argument rules (also used by Rust), not shell
    // escaping: https://learn.microsoft.com/cpp/c-language/parsing-c-command-line-arguments
    command.push('"');
    let mut backslashes = 0;
    for c in arg.chars() {
        if c == '\\' {
            backslashes += 1;
            continue;
        }
        let count = if c == '"' {
            backslashes * 2 + 1
        } else {
            backslashes
        };
        command.extend(std::iter::repeat('\\').take(count));
        command.push(c);
        backslashes = 0;
    }
    // Double trailing backslashes so they cannot escape the closing quote.
    command.extend(std::iter::repeat('\\').take(backslashes * 2));
    command.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_executable_with_and_without_arguments() {
        assert_eq!(
            build_command_line(r"C:\Users\John Doe\app.exe", &[]).unwrap(),
            r#""C:\Users\John Doe\app.exe""#,
        );
        assert_eq!(
            build_command_line(r"C:\app.exe", &["--autostart".into()]).unwrap(),
            r#""C:\app.exe" --autostart"#,
        );
    }

    #[test]
    fn accepts_one_surrounding_quote_pair() {
        for path in [
            r"C:\Users\John Doe\app.exe",
            r"\\server\share name\app.exe",
            "app.exe",
        ] {
            for args in [
                vec![],
                vec!["--autostart".into(), "two words".into(), "".into()],
            ] {
                assert_eq!(
                    build_command_line(&format!("\"{path}\""), &args).unwrap(),
                    build_command_line(path, &args).unwrap(),
                );
            }
        }
    }

    #[test]
    fn preserves_argument_boundaries_and_literals() {
        for (input, expected) in [
            ("", r#""""#),
            ("two words", r#""two words""#),
            ("a\tb", "\"a\tb\""),
            (r#"a"b"#, r#""a\"b""#),
            (r#"a\"b"#, r#""a\\\"b""#),
            (r#"a\\"b"#, r#""a\\\\\"b""#),
            (r"C:\plain\", r"C:\plain\"),
            (r"C:\two words\", r#""C:\two words\\""#),
            (r"C:\two words\\", r#""C:\two words\\\\""#),
            ("雪😀", "雪😀"),
            ("%PATH%&|<>^", "%PATH%&|<>^"),
        ] {
            let actual = build_command_line("app.exe", &[input.into()]).unwrap();
            assert_eq!(actual, format!("\"app.exe\" {expected}"), "{input:?}");
        }
        assert_eq!(
            build_command_line("app.exe", &["".into(), "x".into(), "".into()]).unwrap(),
            r#""app.exe" "" x """#,
        );
    }

    #[test]
    fn rejects_inputs_that_cannot_be_represented() {
        for path in [
            "",
            "app\0.exe",
            "a\"b.exe",
            "\"",
            "\"\"",
            "\"app.exe",
            "app.exe\"",
            "\"\"app.exe\"\"",
            "\"app\0.exe\"",
        ] {
            assert_eq!(
                build_command_line(path, &[]).unwrap_err().kind(),
                io::ErrorKind::InvalidInput,
            );
        }
        assert_eq!(
            build_command_line("app.exe", &["bad\0arg".into()])
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput,
        );
    }
}
