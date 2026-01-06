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

impl TugOfWar {
    pub fn new(energy: Energy) -> TugOfWar {
        TugOfWar { energy }
    }

    pub fn default() -> TugOfWar {
        // Стандартная разбалловка
        Self::new(100)
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
