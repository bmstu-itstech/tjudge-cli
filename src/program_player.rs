use std::io::{BufRead, BufReader, BufWriter, Write, Result};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;
use timeout_readwrite::TimeoutReader;

use crate::game::Player;

const PROGRAM_PLAYER_TIMEOUT: Duration = Duration::from_millis(200);


pub struct ProgramPlayer {
    reader: BufReader<TimeoutReader<ChildStdout>>,
    writer: BufWriter<ChildStdin>,
}

impl ProgramPlayer {
    pub fn new(path: &str) -> Result<ProgramPlayer> {
        let mut cmd = Command::new(path);
        let process = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(ProgramPlayer{
            reader: BufReader::new(
                TimeoutReader::new(process.stdout.unwrap(), PROGRAM_PLAYER_TIMEOUT)
            ),
            writer: BufWriter::new(process.stdin.unwrap()),
        })
    }
}

impl Player for ProgramPlayer {
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

    #[test]
    fn silent_program() {
        let mut player = ProgramPlayer::new("resources/invalid/silent.py").unwrap();
        let res = player.ask();
        assert!(res.is_err());
    }
    
    #[test]
    fn echo_program() {
        let mut player = ProgramPlayer::new("resources/common/echo.py").unwrap();
        assert!(player.say("Hello, world!".to_string()).is_ok());
        let res = player.ask();
        assert!(res.is_ok(), "unexpected error: {}", res.unwrap_err());
        assert_eq!(res.unwrap(), "Hello, world!".to_string());
    }
}
