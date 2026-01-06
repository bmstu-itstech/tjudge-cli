use std::io;

use crate::game::*;
use crate::vprintln;

/// Модифицированная дилемма заключённого.
///
/// Даётся `iters` итераций. На каждой итерации участник может выбрать, предать ли ему соперника
/// (DEFECT) или сотрудничать с ним (COOPERATE). Обозначим как D и C соответственно.
/// В зависимости от выбора участников им начисляются очки:
///     если предадут оба, то они получат `mutual_defects` очков;
///     если один предаст другого, то первый получит `defect` очков, другой `0`;
///     если оба пойдут на сотрудничество, то они получат по `cooperate` очков.
///
/// Модифицированная дилемма заключённого отличается от классической наличием нескольких итераций,
/// причём программы знают предыдущий выбор соперника. Это позволяет строить, например,
/// "мстительные" тактики.
pub struct PrisonerDilemma {
    both_defects: Score,    // Если оба предадут.
    betrayer_reward: Score, // Если один предаст другого.
    both_cooperate: Score,  // Если оба будут сотрудничать.
}

impl Game for PrisonerDilemma {
    fn round(
        &self,
        left: &mut dyn Player,
        right: &mut dyn Player,
        iters: u32,
    ) -> Result<(Score, Score), GameError> {
        let mut left = GameMediator::new(left);
        let mut right = GameMediator::new(right);

        // Сообщаем всем участникам количество итераций.
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

impl PrisonerDilemma {
    pub fn new(mutual_defects: Score, defect: Score, cooperate: Score) -> PrisonerDilemma {
        PrisonerDilemma {
            both_defects: mutual_defects,
            betrayer_reward: defect,
            both_cooperate: cooperate,
        }
    }

    pub fn default() -> PrisonerDilemma {
        // Стандартная разбалловка
        Self::new(1, 10, 5)
    }

    /// Один выбор игроков с последующим ответом.
    fn iteration(
        &self,
        left: &mut GameMediator,
        right: &mut GameMediator,
    ) -> Result<(Score, Score), GameError> {
        let l_decision = left.decision().map_err(GameError::ErrorLeft)?;
        vprintln!("[>] decision: {:?}", l_decision);
        let r_decision = right.decision().map_err(GameError::ErrorRight)?;
        vprintln!("[<] decision: {:?}", r_decision);

        left.notify(&r_decision).map_err(GameError::ErrorLeft)?;
        right.notify(&l_decision).map_err(GameError::ErrorRight)?;

        Ok(match (&l_decision, &r_decision) {
            (Decision::Cooperate, Decision::Cooperate) => {
                (self.both_cooperate, self.both_cooperate)
            }
            (Decision::Cooperate, Decision::Defect) => (0, self.betrayer_reward),
            (Decision::Defect, Decision::Cooperate) => (self.betrayer_reward, 0),
            (Decision::Defect, Decision::Defect) => (self.both_defects, self.both_defects),
        })
    }
}

struct GameMediator<'a> {
    actor: &'a mut dyn Player,
}

impl<'a> GameMediator<'a> {
    fn new(actor: &'a mut dyn Player) -> GameMediator<'a> {
        GameMediator { actor }
    }
}

impl GameMediator<'_> {
    fn initial(&mut self, iters: u32) -> io::Result<()> {
        self.actor.say(format!("{}", iters))
    }

    fn decision(&mut self) -> io::Result<Decision> {
        Decision::from_str(self.actor.ask()?.as_str())
    }

    fn notify(&mut self, d: &Decision) -> io::Result<()> {
        self.actor.say(d.to_str().to_string())
    }
}

#[derive(Debug)]
enum Decision {
    Cooperate,
    Defect,
}

impl Decision {
    fn from_str(s: &str) -> io::Result<Self> {
        match s {
            "COOPERATE" => Ok(Decision::Cooperate),
            "DEFECT" => Ok(Decision::Defect),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "unknown action '{}', expected one of ['COOPERATE', 'DEFECT']",
                    s
                ),
            )),
        }
    }

    fn to_str(&self) -> &'static str {
        match self {
            Decision::Cooperate => "COOPERATE",
            Decision::Defect => "DEFECT",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Result;

    struct TitForTatPlayer {
        started: bool,
        next_choice: String,
    }

    impl TitForTatPlayer {
        fn new(first_choice: &str) -> Self {
            TitForTatPlayer {
                started: false,
                next_choice: first_choice.to_string(),
            }
        }
    }

    impl Player for TitForTatPlayer {
        fn ask(&mut self) -> Result<String> {
            Ok(self.next_choice.clone())
        }

        fn say(&mut self, s: String) -> Result<()> {
            if !self.started {
                self.started = true;
            } else {
                self.next_choice = s;
            }
            Ok(())
        }
    }

    #[test]
    fn dilemma_with_two_tit_for_tat_players() {
        // Игроки сыграют в ничью:
        //   LEFT | RIGHT |
        // -------+-------|
        //    C   |   D   |   +10    0
        //    D   |   C   |     0  +10
        // ----------------------------
        //                    +10  +10
        let mut l = TitForTatPlayer::new("COOPERATE");
        let mut r = TitForTatPlayer::new("DEFECT");
        let d_game = PrisonerDilemma::new(1, 10, 5);

        let res = d_game.round(&mut l, &mut r, 2);

        assert!(res.is_ok(), "unexpected error: {:?}", res.err().unwrap());
        let res = res.unwrap();
        assert_eq!(10, res.0);
        assert_eq!(10, res.1);
    }

    #[test]
    fn dilemma_invalid_output() {
        let mut l = TitForTatPlayer::new("Sth"); // Выведет это же
        let mut r = TitForTatPlayer::new("DEFECT");
        let d_game = PrisonerDilemma::new(1, 10, 5);

        let res = d_game.round(&mut l, &mut r, 2);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorLeft(_)));
    }
}
