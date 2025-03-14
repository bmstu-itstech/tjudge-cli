use std::io;

/// Участник игры, который может:
/// - отправить что-то вовне (ask);
/// - что-то получить извне (say).
pub trait Player {
    fn ask(&mut self) -> io::Result<String>;
    fn say(&mut self, s: String) -> io::Result<()>;
}

pub type Score = i32;

#[derive(Debug)]
pub enum GameError {
    ErrorLeft(io::Error),
    ErrorRight(io::Error),
}

pub trait Game {
    /// Играет один раунд меду двумя игроками с заданным количеством итераций.
    /// Возвращает набранный счёт игроками в порядке следования аргументов.
    fn round<T1, T2>(
        &self, left: &mut T1, right: &mut T2, iters: u32
    ) -> Result<(Score, Score), GameError>
    where
        T1: Player,
        T2: Player;
}
