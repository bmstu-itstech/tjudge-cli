use std::cmp::Ordering;
use std::io;

use crate::game::*;
use crate::vprintln;

type Energy = u32;

/// Игра на перетягивание каната в теории игр.
///
/// Участникам даётся `energy` сил. Известно количество итераций `iters`. На каждой итерации
/// участник выбирает, сколько ему сил (`energy`) потратить.
/// Назовём участников `A` и `B`. Пусть в итерации они выбрали потратить `a` и `b` сил. Тогда:
///     если `a` > `b`, то участник `A` получает 1 балл;
///     если `a` < `b`, то участник `B` получает 1 балл;
///     если `a` = `b`, то никто не получает баллы.
/// Нельзя потратить больше сил, чем осталось у участника.
pub struct TugOfWar {
    energy: Energy,
}

impl Game for TugOfWar {
    fn round(
        &self,
        left: &mut dyn Player,
        right: &mut dyn Player,
        iters: u32,
    ) -> Result<(Score, Score), GameError> {
        let mut left = GameMediator::new(left, self.energy);
        let mut right = GameMediator::new(right, self.energy);

        // Сообщаем всем участникам количество сил и количество итераций.
        vprintln!("[init] iterations: {iters}");
        left.initial(iters).map_err(GameError::ErrorLeft)?;
        right.initial(iters).map_err(GameError::ErrorRight)?;

        let mut score: (Score, Score) = (0, 0);
        for i in 0..iters {
            let res = self.iteration(&mut left, &mut right)?;
            vprintln!("[iter-{i:02}] result: {res:?}");
            score.0 += res.0;
            score.1 += res.1;
            vprintln!("[iter-{i:02}] score: {score:?}");
        }

        vprintln!("[result] score: {score:?}");
        Ok(score)
    }
}

impl Default for TugOfWar {
    fn default() -> Self {
        // Стандартные параметры (energy = 100)
        Self::new(100)
    }
}

impl TugOfWar {
    pub fn new(energy: Energy) -> TugOfWar {
        TugOfWar { energy }
    }

    fn iteration(
        &self,
        left: &mut GameMediator,
        right: &mut GameMediator,
    ) -> Result<(Score, Score), GameError> {
        let l_spent = left.pull().map_err(GameError::ErrorLeft)?;
        vprintln!("[>] pull: {l_spent}");
        let r_spent = right.pull().map_err(GameError::ErrorRight)?;
        vprintln!("[<] pull: {r_spent}");

        left.notify(r_spent).map_err(GameError::ErrorLeft)?;
        right.notify(l_spent).map_err(GameError::ErrorRight)?;

        Ok(match l_spent.cmp(&r_spent) {
            Ordering::Less => (0, 1),
            Ordering::Greater => (1, 0),
            Ordering::Equal => (0, 0),
        })
    }
}

struct GameMediator<'a> {
    actor: &'a mut dyn Player,
    energy: Energy,
}

impl<'a> GameMediator<'a> {
    fn new(actor: &'a mut dyn Player, energy: u32) -> GameMediator<'a> {
        GameMediator { actor, energy }
    }
}

impl GameMediator<'_> {
    fn initial(&mut self, iters: u32) -> io::Result<()> {
        self.actor.say(format!("{}", self.energy))?;
        self.actor.say(format!("{}", iters))
    }

    fn pull(&mut self) -> io::Result<Energy> {
        let spent = self
            .actor
            .ask()?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        if spent > self.energy {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("expected spent <= energy, got {} > {}", spent, self.energy),
            ));
        };
        self.energy -= spent;

        Ok(spent)
    }

    fn notify(&mut self, another_spent: Energy) -> io::Result<()> {
        self.actor.say(format!("{}", another_spent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::io::Result;

    /// Скриптованный игрок: возвращает заготовленные ответы на `ask()` и игнорирует `say()`.
    struct ScriptedPlayer {
        answers: VecDeque<String>,
    }

    impl ScriptedPlayer {
        fn new(answers: &[&str]) -> Self {
            ScriptedPlayer {
                answers: answers.iter().map(|s| s.to_string()).collect(),
            }
        }
    }

    impl Player for ScriptedPlayer {
        fn ask(&mut self) -> Result<String> {
            Ok(self.answers.pop_front().expect("ScriptedPlayer: закончились заготовленные ответы"))
        }

        fn say(&mut self, _s: String) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn higher_spend_wins_round() {
        // Левый тратит 3, правый 1 => левый выигрывает
        let mut l = ScriptedPlayer::new(&["3"]);
        let mut r = ScriptedPlayer::new(&["1"]);
        let game = TugOfWar::new(10);

        let res = game.round(&mut l, &mut r, 1);
        assert!(res.is_ok(), "unexpected error: {:?}", res.err().unwrap());
        assert_eq!(res.unwrap(), (1, 0));
    }

    #[test]
    fn equal_spend_is_draw() {
        // Оба тратят 5 => ничья, 0 очков
        let mut l = ScriptedPlayer::new(&["5"]);
        let mut r = ScriptedPlayer::new(&["5"]);
        let game = TugOfWar::new(10);

        let res = game.round(&mut l, &mut r, 1);
        assert!(res.is_ok(), "unexpected error: {:?}", res.err().unwrap());
        assert_eq!(res.unwrap(), (0, 0));
    }

    #[test]
    fn exceed_energy_is_error() {
        // Левый пытается потратить 20 при 10 энергии => ошибка
        let mut l = ScriptedPlayer::new(&["20"]);
        let mut r = ScriptedPlayer::new(&["1"]);
        let game = TugOfWar::new(10);

        let res = game.round(&mut l, &mut r, 1);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorLeft(_)));
    }

    #[test]
    fn right_player_exceed_energy_is_error() {
        // Правый пытается потратить 20 при 10 энергии => ErrorRight
        let mut l = ScriptedPlayer::new(&["1"]);
        let mut r = ScriptedPlayer::new(&["20"]);
        let game = TugOfWar::new(10);

        let res = game.round(&mut l, &mut r, 1);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorRight(_)));
    }

    #[test]
    fn energy_depletes_across_iterations() {
        // energy=10, левый тратит [6, 5] => вторая итерация провалится (осталось только 4)
        let mut l = ScriptedPlayer::new(&["6", "5"]);
        let mut r = ScriptedPlayer::new(&["1", "1"]);
        let game = TugOfWar::new(10);

        let res = game.round(&mut l, &mut r, 2);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorLeft(_)));
    }

    #[test]
    fn invalid_input_is_error() {
        // Левый отправляет нечисловой ввод => ошибка парсинга
        let mut l = ScriptedPlayer::new(&["abc"]);
        let mut r = ScriptedPlayer::new(&["1"]);
        let game = TugOfWar::new(10);

        let res = game.round(&mut l, &mut r, 1);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorLeft(_)));
    }

    #[test]
    fn score_accumulates_over_iterations() {
        // 3 итерации: левый тратит [5, 3, 1], правый тратит [1, 3, 5]
        // итер. 0: 5>1 => (1,0)
        // итер. 1: 3=3 => (0,0)
        // итер. 2: 1<5 => (0,1)
        // итого: (1, 1)
        let mut l = ScriptedPlayer::new(&["5", "3", "1"]);
        let mut r = ScriptedPlayer::new(&["1", "3", "5"]);
        let game = TugOfWar::new(100);

        let res = game.round(&mut l, &mut r, 3);
        assert!(res.is_ok(), "unexpected error: {:?}", res.err().unwrap());
        assert_eq!(res.unwrap(), (1, 1));
    }
}
