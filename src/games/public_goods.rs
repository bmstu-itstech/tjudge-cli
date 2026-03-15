use std::io;

use crate::game::*;
use crate::vprintln;

/// Игра "Общественное благо".
///
/// У каждого игрока в начале каждой итерации есть `endowment` токенов.
/// Оба одновременно выбирают, сколько вложить в общий пул (от 0 до `endowment`).
/// Пул умножается на `multiplier` и делится поровну.
/// Невложенные токены остаются у игрока.
///
/// Выплата: `(endowment - c_i) + multiplier * (c_1 + c_2) / 2`.
/// Результат усекается до целого числа.
///
/// Дилемма: если оба вложат всё — каждый получит `multiplier * endowment`.
/// Равновесие Нэша: не вкладывать ничего (каждый получит `endowment`).
pub struct PublicGoods {
    endowment: u32,
    multiplier: f64,
}

impl Game for PublicGoods {
    fn round(
        &self,
        left: &mut dyn Player,
        right: &mut dyn Player,
        iters: u32,
    ) -> Result<(Score, Score), GameError> {
        let mut left = GameMediator::new(left, self.endowment);
        let mut right = GameMediator::new(right, self.endowment);

        vprintln!(
            "[init] endowment: {}, multiplier: {}, iterations: {iters}",
            self.endowment,
            self.multiplier
        );
        left.initial(self.endowment, self.multiplier, iters)
            .map_err(GameError::ErrorLeft)?;
        right
            .initial(self.endowment, self.multiplier, iters)
            .map_err(GameError::ErrorRight)?;

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

impl PublicGoods {
    pub fn new(endowment: u32, multiplier: f64) -> PublicGoods {
        PublicGoods {
            endowment,
            multiplier,
        }
    }

    pub fn default() -> PublicGoods {
        Self::new(20, 1.5)
    }

    fn iteration(
        &self,
        left: &mut GameMediator,
        right: &mut GameMediator,
    ) -> Result<(Score, Score), GameError> {
        let l_contrib = left.contribution().map_err(GameError::ErrorLeft)?;
        vprintln!("[>] contribution: {l_contrib}");
        let r_contrib = right.contribution().map_err(GameError::ErrorRight)?;
        vprintln!("[<] contribution: {r_contrib}");

        left.notify(r_contrib).map_err(GameError::ErrorLeft)?;
        right.notify(l_contrib).map_err(GameError::ErrorRight)?;

        let pool = self.multiplier * (l_contrib + r_contrib) as f64 / 2.0;
        let l_payoff = (self.endowment - l_contrib) as f64 + pool;
        let r_payoff = (self.endowment - r_contrib) as f64 + pool;

        Ok((l_payoff as Score, r_payoff as Score))
    }
}

struct GameMediator<'a> {
    actor: &'a mut dyn Player,
    endowment: u32,
}

impl<'a> GameMediator<'a> {
    fn new(actor: &'a mut dyn Player, endowment: u32) -> GameMediator<'a> {
        GameMediator { actor, endowment }
    }
}

impl GameMediator<'_> {
    fn initial(&mut self, endowment: u32, multiplier: f64, iters: u32) -> io::Result<()> {
        self.actor.say(format!("{}", endowment))?;
        let m_str = format!("{}", multiplier);
        let m_str = if m_str.contains('.') { m_str } else { format!("{m_str}.0") };
        self.actor.say(m_str)?;
        self.actor.say(format!("{}", iters))
    }

    fn contribution(&mut self) -> io::Result<u32> {
        let contrib: u32 = self
            .actor
            .ask()?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        if contrib > self.endowment {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "expected contribution <= endowment, got {} > {}",
                    contrib, self.endowment
                ),
            ));
        }

        Ok(contrib)
    }

    fn notify(&mut self, opponent_contribution: u32) -> io::Result<()> {
        self.actor.say(format!("{}", opponent_contribution))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Result;

    struct ConstantContributor {
        started: bool,
        contribution: u32,
    }

    impl ConstantContributor {
        fn new(contribution: u32) -> Self {
            ConstantContributor {
                started: false,
                contribution,
            }
        }
    }

    impl Player for ConstantContributor {
        fn ask(&mut self) -> Result<String> {
            Ok(format!("{}", self.contribution))
        }

        fn say(&mut self, _s: String) -> Result<()> {
            if !self.started {
                self.started = true;
            }
            Ok(())
        }
    }

    #[test]
    fn both_contribute_zero() {
        let mut l = ConstantContributor::new(0);
        let mut r = ConstantContributor::new(0);
        let game = PublicGoods::new(20, 1.5);

        let res = game.round(&mut l, &mut r, 1).unwrap();
        // (20-0) + 1.5*(0+0)/2 = 20
        assert_eq!(res, (20, 20));
    }

    #[test]
    fn both_contribute_all() {
        let mut l = ConstantContributor::new(20);
        let mut r = ConstantContributor::new(20);
        let game = PublicGoods::new(20, 1.5);

        let res = game.round(&mut l, &mut r, 1).unwrap();
        // (20-20) + 1.5*(20+20)/2 = 0 + 30 = 30
        assert_eq!(res, (30, 30));
    }

    #[test]
    fn free_rider_advantage() {
        let mut l = ConstantContributor::new(0);
        let mut r = ConstantContributor::new(20);
        let game = PublicGoods::new(20, 1.5);

        let res = game.round(&mut l, &mut r, 1).unwrap();
        // left: (20-0) + 1.5*20/2 = 20 + 15 = 35
        // right: (20-20) + 1.5*20/2 = 0 + 15 = 15
        assert_eq!(res, (35, 15));
    }

    #[test]
    fn multiple_iterations() {
        let mut l = ConstantContributor::new(10);
        let mut r = ConstantContributor::new(10);
        let game = PublicGoods::new(20, 1.5);

        let res = game.round(&mut l, &mut r, 3).unwrap();
        // per iter: (20-10) + 1.5*20/2 = 10 + 15 = 25
        assert_eq!(res, (75, 75));
    }

    #[test]
    fn contribution_exceeds_endowment() {
        let mut l = ConstantContributor::new(21);
        let mut r = ConstantContributor::new(10);
        let game = PublicGoods::new(20, 1.5);

        let res = game.round(&mut l, &mut r, 1);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorLeft(_)));
    }

    struct InvalidPlayer;

    impl Player for InvalidPlayer {
        fn ask(&mut self) -> Result<String> {
            Ok("abc".to_string())
        }

        fn say(&mut self, _s: String) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn non_numeric_contribution() {
        let mut l = InvalidPlayer;
        let mut r = ConstantContributor::new(10);
        let game = PublicGoods::new(20, 1.5);

        let res = game.round(&mut l, &mut r, 1);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorLeft(_)));
    }
}
