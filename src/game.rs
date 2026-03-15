use std::io;

/// Участник игры, который может:
/// - ответить на запрос судьи (ask) -- судья считывает ответ игрока;
/// - принять данные от судьи (say) -- судья отправляет данные игроку.
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
    /// Играет один раунд между двумя игроками с заданным количеством итераций.
    /// Возвращает набранный счёт игроками в порядке следования аргументов.
    fn round(
        &self,
        left: &mut dyn Player,
        right: &mut dyn Player,
        iters: u32,
    ) -> Result<(Score, Score), GameError>;
}
