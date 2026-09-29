use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use std::ffi::OsStr;
use std::io::{BufRead, BufReader, BufWriter, Error, ErrorKind, Result, Write};
use std::os::fd::AsFd;
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;
use timeout_readwrite::{TimeoutReader, TimeoutWriter};

use crate::game::Player;

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(200);
// первый ответ включает запуск программы: интерпретатору или JVM на
// загруженной машине не хватает 200 мс, дальше ходы снова по DEFAULT_TIMEOUT
const STARTUP_TIMEOUT: Duration = Duration::from_secs(2);

pub struct SubprocessPlayer {
    child: Child,
    reader: BufReader<TimeoutReader<ChildStdout>>,
    writer: BufWriter<TimeoutWriter<ChildStdin>>,
    started: bool,
}

impl SubprocessPlayer {
    pub fn from_program(program: impl AsRef<OsStr>) -> Result<Self> {
        let cmd = Command::new(program);
        Self::new(cmd, DEFAULT_TIMEOUT)
    }

    #[allow(unused)]
    pub fn from_script(interpreter: impl AsRef<OsStr>, script: impl AsRef<OsStr>) -> Result<Self> {
        if !Path::new(&script).exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("script not exists: {}", script.as_ref().to_string_lossy()),
            ));
        }
        let mut cmd = Command::new(interpreter);
        cmd.arg(script);
        Self::new(cmd, DEFAULT_TIMEOUT)
    }

    pub fn new(mut cmd: Command, timeout: Duration) -> Result<SubprocessPlayer> {
        let mut process = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = process.stdout.take().ok_or_else(|| {
            Error::new(ErrorKind::Other, "failed to capture subprocess stdout")
        })?;
        let stdin = process.stdin.take().ok_or_else(|| {
            Error::new(ErrorKind::Other, "failed to capture subprocess stdin")
        })?;

        Ok(SubprocessPlayer {
            child: process,
            reader: BufReader::new(TimeoutReader::new(stdout, timeout)),
            writer: BufWriter::new(TimeoutWriter::new(stdin, timeout)),
            started: false,
        })
    }
}

impl Drop for SubprocessPlayer {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {} // процесс уже завершился
            Ok(None) => {
                if let Err(e) = self.child.kill() {
                    crate::vprintln!("[drop] failed to kill child process: {e}");
                }
                if let Err(e) = self.child.wait() {
                    crate::vprintln!("[drop] failed to reap child process: {e}");
                }
            }
            Err(e) => {
                crate::vprintln!("[drop] failed to check child process status: {e}");
            }
        }
    }
}

impl Player for SubprocessPlayer {
    fn ask(&mut self) -> Result<String> {
        if !self.started {
            self.started = true;
            if self.reader.buffer().is_empty() {
                wait_readable(self.reader.get_ref(), STARTUP_TIMEOUT)?;
            }
        }
        let mut line = String::new();
        let n = self.reader.read_line(&mut line)?;
        if n == 0 {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "subprocess terminated unexpectedly",
            ));
        }
        Ok(line.trim_end().to_string())
    }

    fn say(&mut self, s: String) -> Result<()> {
        self.writer
            .write_all(s.as_bytes())
            .and_then(|_| self.writer.write_all("\n".as_bytes()))
            .and_then(|_| self.writer.flush())
    }
}

// ждёт данных в stdout программы; текст ошибки как у timeout_readwrite
fn wait_readable(fd: &impl AsFd, timeout: Duration) -> Result<()> {
    let mut fds = [PollFd::new(fd.as_fd(), PollFlags::POLLIN)];
    let timeout = PollTimeout::try_from(timeout).map_err(Error::other)?;
    if poll(&mut fds, timeout).map_err(Error::other)? == 0 {
        return Err(Error::new(
            ErrorKind::TimedOut,
            "timed out waiting for fd to be ready",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const PYTHON_CMD: &str = "python3";

    fn python_script_player(directory: &str, script: &str) -> Result<SubprocessPlayer> {
        SubprocessPlayer::from_script(
            PYTHON_CMD,
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join(directory)
                .join(script),
        )
    }

    #[test]
    fn silent_program() {
        let mut player = python_script_player("invalid", "silent.py").unwrap();
        let res = player.ask();
        assert!(res.is_err());
    }

    #[test]
    fn echo_program() {
        let mut player = python_script_player("common", "echo.py").unwrap();
        assert!(player.say("Hello, world!".to_string()).is_ok());
        let res = player.ask();
        assert!(res.is_ok(), "unexpected error: {}", res.unwrap_err());
        assert_eq!(res.unwrap(), "Hello, world!".to_string());
    }

    fn shell_player(script: &str) -> SubprocessPlayer {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", script]);
        SubprocessPlayer::new(cmd, DEFAULT_TIMEOUT).unwrap()
    }

    #[test]
    fn slow_start_fits_startup_timeout() {
        let mut player = shell_player("sleep 0.5; echo first; read x; echo second");
        assert_eq!(player.ask().unwrap(), "first");
        assert!(player.say("go".to_string()).is_ok());
        assert_eq!(player.ask().unwrap(), "second");
    }

    #[test]
    fn slow_move_after_start_times_out() {
        let mut player = shell_player("echo first; read x; sleep 0.5; echo second");
        assert_eq!(player.ask().unwrap(), "first");
        assert!(player.say("go".to_string()).is_ok());
        let err = player.ask().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::TimedOut);
    }

    #[test]
    fn non_existent_program() {
        let res = SubprocessPlayer::from_program("python4");
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), ErrorKind::NotFound);
    }

    #[test]
    fn non_existent_script() {
        let res = SubprocessPlayer::from_script(PYTHON_CMD, "_");
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), ErrorKind::NotFound);
    }
}
