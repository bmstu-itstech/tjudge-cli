use std::ffi::OsStr;
use std::io::{BufRead, BufReader, BufWriter, Error, ErrorKind, Result, Write};
use std::path::Path;
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;
use timeout_readwrite::TimeoutReader;

use crate::game::Player;

const DEFAULT_TIMEOUT: Duration = Duration::from_millis(200);

pub struct SubprocessPlayer {
    reader: BufReader<TimeoutReader<ChildStdout>>,
    writer: BufWriter<ChildStdin>,
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
        let process = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(SubprocessPlayer {
            reader: BufReader::new(TimeoutReader::new(process.stdout.unwrap(), timeout)),
            writer: BufWriter::new(process.stdin.unwrap()),
        })
    }
}

impl Player for SubprocessPlayer {
    fn ask(&mut self) -> Result<String> {
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map(|_| line.trim_end().to_string())
    }

    fn say(&mut self, s: String) -> Result<()> {
        self.writer
            .write_all(s.as_bytes())
            .and_then(|_| self.writer.write_all("\n".as_bytes()))
            .and_then(|_| self.writer.flush())
    }
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
