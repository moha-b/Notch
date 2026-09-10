use super::Failure;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const MAX_TOKEN_BYTES: u64 = 16_384;

pub(super) fn token() -> Result<String, Failure> {
    let mut command = Command::new("gh");
    command.args(["auth", "token", "--hostname", "github.com"]);
    credential_output(&mut command, Duration::from_secs(5))
}

fn credential_output(command: &mut Command, timeout: Duration) -> Result<String, Failure> {
    let mut running = spawn_credential_process(command)?;
    let stdout = running
        .0
        .stdout
        .take()
        .ok_or(Failure::Invalid("Credential output is unavailable."))?;
    let output = read_stdout(stdout);
    let deadline = Instant::now() + timeout;
    wait_success(&mut running.0, deadline)?;
    let bytes = output
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|_| Failure::Unavailable("GitHub credential lookup timed out.".into()))?
        .map_err(|_| Failure::Unavailable("Could not read GitHub credential output.".into()))?;
    parse_token(bytes)
}

fn spawn_credential_process(command: &mut Command) -> Result<CredentialProcess, Failure> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
        .spawn()
        .map(CredentialProcess)
        .map_err(|_| Failure::NeedsAuth)
}

fn read_stdout(
    stdout: std::process::ChildStdout,
) -> std::sync::mpsc::Receiver<std::io::Result<Vec<u8>>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let read = stdout
            .take(MAX_TOKEN_BYTES + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        // The caller may already have timed out and stopped the process.
        let _ = sender.send(read);
    });
    receiver
}

fn wait_success(child: &mut Child, deadline: Instant) -> Result<(), Failure> {
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| Failure::Unavailable("Could not wait for GitHub CLI.".into()))?
        {
            return if status.success() {
                Ok(())
            } else {
                Err(Failure::NeedsAuth)
            };
        }
        if Instant::now() >= deadline {
            return Err(Failure::Unavailable(
                "GitHub credential lookup timed out.".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn parse_token(bytes: Vec<u8>) -> Result<String, Failure> {
    if bytes.len() as u64 > MAX_TOKEN_BYTES {
        return Err(Failure::Invalid("GitHub credential output is too large."));
    }
    let token = String::from_utf8(bytes)
        .map_err(|_| Failure::Invalid("GitHub credential output is not UTF-8."))?;
    let token = token.trim();
    if token.is_empty() {
        return Err(Failure::NeedsAuth);
    }
    if token.chars().any(char::is_whitespace) {
        return Err(Failure::Invalid(
            "GitHub credential output contains whitespace.",
        ));
    }
    Ok(token.into())
}

struct CredentialProcess(Child);

impl Drop for CredentialProcess {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(Some(_))) {
            return;
        }
        if let Err(error) = self.0.kill() {
            crate::applog(&format!("Could not stop GitHub credential helper: {error}"));
            return;
        }
        if let Err(error) = self.0.wait() {
            crate::applog(&format!("Could not reap GitHub credential helper: {error}"));
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn invalid_or_oversized_credential_output_is_never_used_as_a_key() {
        for bytes in [
            vec![],
            vec![0xff],
            b"two tokens".to_vec(),
            vec![b'x'; MAX_TOKEN_BYTES as usize + 1],
        ] {
            assert!(parse_token(bytes).is_err());
        }
    }

    #[test]
    fn credential_process_returns_token_and_reports_nonzero_exit() {
        for (script, expected) in [
            ("echo fixture-token", Some("fixture-token")),
            ("exit /b 1", None),
        ] {
            let mut command = Command::new("cmd.exe");
            command.args(["/D", "/C", script]);
            let credential = credential_output(&mut command, Duration::from_secs(5));
            match expected {
                Some(token) => assert_eq!(credential.unwrap(), token),
                None => assert!(matches!(credential, Err(Failure::NeedsAuth))),
            }
        }
    }

    #[test]
    fn stalled_credential_helper_is_stopped_before_polling_can_freeze() {
        let mut command = Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Start-Sleep -Seconds 30",
        ]);
        let started = Instant::now();
        assert!(matches!(
            credential_output(&mut command, Duration::from_millis(200)),
            Err(Failure::Unavailable(_))
        ));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
