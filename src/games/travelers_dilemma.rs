use std::io;

use crate::game::*;
use crate::vprintln;

/// Дилемма путешественника.
///
/// Два путешественника потеряли одинаковые чемоданы. Авиакомпания просит каждого независимо
/// назвать стоимость в диапазоне `[lower, upper]`.
/// Если оба назвали одинаковое число — оба получают это число.
/// Если значения отличаются — оба получают минимум из двух значений,
/// при этом назвавший меньше получает бонус `+reward`,
/// а назвавший больше получает штраф `-reward`.
///
/// Равновесие Нэша: оба называют `lower`, но кооперативная игра часто даёт ~90.
pub struct TravelersDilemma {
    lower: Score,  // Нижняя граница (L)
    upper: Score,  // Верхняя граница (U)
    reward: Score, // Бонус/штраф (R)
}

impl Game for TravelersDilemma {
    fn round(
        &self,
        left: &mut dyn Player,
        right: &mut dyn Player,
        iters: u32,
    ) -> Result<(Score, Score), GameError> {
        let mut left = GameMediator::new(left, self.lower, self.upper);
        let mut right = GameMediator::new(right, self.lower, self.upper);

        vprintln!("[init] L: {}, U: {}, R: {}, iterations: {iters}", self.lower, self.upper, self.reward);
        left.initial(self.lower, self.upper, self.reward, iters)
            .map_err(GameError::ErrorLeft)?;
        right.initial(self.lower, self.upper, self.reward, iters)
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

impl TravelersDilemma {
    pub fn new(lower: Score, upper: Score, reward: Score) -> TravelersDilemma {
        TravelersDilemma {
            lower,
            upper,
            reward,
        }
    }

    pub fn default() -> TravelersDilemma {
        Self::new(2, 100, 2)
    }

    fn iteration(
        &self,
        left: &mut GameMediator,
        right: &mut GameMediator,
    ) -> Result<(Score, Score), GameError> {
        let l_claim = left.claim().map_err(GameError::ErrorLeft)?;
        vprintln!("[>] claim: {l_claim}");
        let r_claim = right.claim().map_err(GameError::ErrorRight)?;
        vprintln!("[<] claim: {r_claim}");

        left.notify(r_claim).map_err(GameError::ErrorLeft)?;
        right.notify(l_claim).map_err(GameError::ErrorRight)?;

        Ok(if l_claim == r_claim {
            (l_claim, r_claim)
        } else {
            let min_val = l_claim.min(r_claim);
            if l_claim < r_claim {
                (min_val + self.reward, min_val - self.reward)
            } else {
                (min_val - self.reward, min_val + self.reward)
            }
        })
    }
}

struct GameMediator<'a> {
    actor: &'a mut dyn Player,
    lower: Score,
    upper: Score,
}

impl<'a> GameMediator<'a> {
    fn new(actor: &'a mut dyn Player, lower: Score, upper: Score) -> GameMediator<'a> {
        GameMediator {
            actor,
            lower,
            upper,
        }
    }
}

impl GameMediator<'_> {
    fn initial(
        &mut self,
        lower: Score,
        upper: Score,
        reward: Score,
        iters: u32,
    ) -> io::Result<()> {
        self.actor.say(format!("{}", lower))?;
        self.actor.say(format!("{}", upper))?;
        self.actor.say(format!("{}", reward))?;
        self.actor.say(format!("{}", iters))
    }

    fn claim(&mut self) -> io::Result<Score> {
        let claim: Score = self
            .actor
            .ask()?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        if claim < self.lower || claim > self.upper {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "expected claim in [{}, {}], got {}",
                    self.lower, self.upper, claim
                ),
            ));
        }

        Ok(claim)
    }

    fn notify(&mut self, opponent_claim: Score) -> io::Result<()> {
        self.actor.say(format!("{}", opponent_claim))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Result;

    struct ConstantClaimPlayer {
        started: bool,
        claim: Score,
    }

    impl ConstantClaimPlayer {
        fn new(claim: Score) -> Self {
            ConstantClaimPlayer {
                started: false,
                claim,
            }
        }
    }

    impl Player for ConstantClaimPlayer {
        fn ask(&mut self) -> Result<String> {
            Ok(format!("{}", self.claim))
        }

        fn say(&mut self, _s: String) -> Result<()> {
            if !self.started {
                self.started = true;
            }
            Ok(())
        }
    }

    #[test]
    fn both_claim_same() {
        let mut l = ConstantClaimPlayer::new(50);
        let mut r = ConstantClaimPlayer::new(50);
        let game = TravelersDilemma::new(2, 100, 2);

        let res = game.round(&mut l, &mut r, 3);
        assert!(res.is_ok(), "unexpected error: {:?}", res.err().unwrap());
        let res = res.unwrap();
        assert_eq!(res, (150, 150));
    }

    #[test]
    fn different_claims_left_lower() {
        let mut l = ConstantClaimPlayer::new(30);
        let mut r = ConstantClaimPlayer::new(70);
        let game = TravelersDilemma::new(2, 100, 2);

        let res = game.round(&mut l, &mut r, 1).unwrap();
        // min=30, left gets 30+2=32, right gets 30-2=28
        assert_eq!(res, (32, 28));
    }

    #[test]
    fn different_claims_right_lower() {
        let mut l = ConstantClaimPlayer::new(70);
        let mut r = ConstantClaimPlayer::new(30);
        let game = TravelersDilemma::new(2, 100, 2);

        let res = game.round(&mut l, &mut r, 1).unwrap();
        // min=30, left gets 30-2=28, right gets 30+2=32
        assert_eq!(res, (28, 32));
    }

    #[test]
    fn both_claim_minimum_nash() {
        let mut l = ConstantClaimPlayer::new(2);
        let mut r = ConstantClaimPlayer::new(2);
        let game = TravelersDilemma::new(2, 100, 2);

        let res = game.round(&mut l, &mut r, 1).unwrap();
        assert_eq!(res, (2, 2));
    }

    #[test]
    fn claim_out_of_range_too_low() {
        let mut l = ConstantClaimPlayer::new(1);
        let mut r = ConstantClaimPlayer::new(50);
        let game = TravelersDilemma::new(2, 100, 2);

        let res = game.round(&mut l, &mut r, 1);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorLeft(_)));
    }

    #[test]
    fn claim_out_of_range_too_high() {
        let mut l = ConstantClaimPlayer::new(50);
        let mut r = ConstantClaimPlayer::new(101);
        let game = TravelersDilemma::new(2, 100, 2);

        let res = game.round(&mut l, &mut r, 1);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorRight(_)));
    }

    struct InvalidPlayer;

    impl Player for InvalidPlayer {
        fn ask(&mut self) -> Result<String> {
            Ok("garbage".to_string())
        }

        fn say(&mut self, _s: String) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn non_numeric_claim() {
        let mut l = InvalidPlayer;
        let mut r = ConstantClaimPlayer::new(50);
        let game = TravelersDilemma::new(2, 100, 2);

        let res = game.round(&mut l, &mut r, 1);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorLeft(_)));
    }
}
