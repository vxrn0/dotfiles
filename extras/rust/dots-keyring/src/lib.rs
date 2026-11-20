use std::env;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

pub fn is_unlocked() -> i32 {
    let output = Command::new("busctl")
        .args([
            "--user",
            "get-property",
            "org.freedesktop.secrets",
            "/org/freedesktop/secrets/collection/login",
            "org.freedesktop.Secret.Collection",
            "Locked",
        ])
        .output();
    let state = output
        .ok()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .trim_end_matches('\n')
                .to_string()
        })
        .unwrap_or_default();
    if state == "b false" {
        eprintln!("Keyring is unlocked");
        0
    } else {
        eprintln!("Keyring is locked");
        1
    }
}

pub fn unlock() -> i32 {
    if is_unlocked() == 0 {
        return 1;
    }

    let mut password = env::var("UNLOCK_PASSWORD").unwrap_or_default();
    if password.is_empty() {
        eprint!("Login password: ");
        let _ = std::io::stderr().flush();
        match read_password() {
            Some(value) => password = value,
            None => {
                eprintln!("No password given");
                return 1;
            }
        }
    }
    if password.is_empty() {
        eprintln!("Empty password");
        return 1;
    }

    let user = match Command::new("whoami").stderr(Stdio::inherit()).output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .trim_end_matches('\n')
            .to_string(),
        Err(error) => {
            eprintln!("dots-keyring: cannot run whoami: {error}");
            String::new()
        }
    };
    let _ = Command::new("killall")
        .args(["-q", "-u", &user, "gnome-keyring-daemon"])
        .status();

    match Command::new("gnome-keyring-daemon")
        .args(["--daemonize", "--login"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(password.as_bytes());
            }
            let mut child_output = Vec::new();
            if let Some(mut stdout) = child.stdout.take() {
                let _ = stdout.read_to_end(&mut child_output);
            }
            let _ = child.wait();
        }
        Err(error) => eprintln!("dots-keyring: cannot run gnome-keyring-daemon: {error}"),
    }
    eprintln!();
    0
}

fn read_password() -> Option<String> {
    let mut restore = None;
    unsafe {
        let mut term = std::mem::zeroed::<libc::termios>();
        if libc::tcgetattr(0, &mut term) == 0 {
            let original = term;
            term.c_lflag &= !libc::ECHO;
            if libc::tcsetattr(0, libc::TCSANOW, &term) == 0 {
                restore = Some(original);
            }
        }
    }
    let mut line = String::new();
    let result = std::io::stdin().read_line(&mut line);
    if let Some(original) = restore {
        unsafe {
            libc::tcsetattr(0, libc::TCSANOW, &original);
        }
    }
    match result {
        Ok(0) | Err(_) => None,
        Ok(_) => {
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            Some(line.trim_matches([' ', '\t']).to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_environment_password_is_empty() {
        env::remove_var("UNLOCK_PASSWORD");
        assert!(env::var("UNLOCK_PASSWORD").unwrap_or_default().is_empty());
    }
}
