use std::io;
use crate::game::{Game, GameError, Player, Score};

/// Модифицированная дилемма заключённого.
/// 
/// Даётся `iters` итераций. На каждой итерации участник может выбрать, предать ли ему соперника
/// (BETRAY) или сотрудничать с ним (COOPERATE). Обозначим как B и C соответственно.
/// В зависимости от выбора участников им начисляются очки:
///     если предадут оба, то они получат `mutual_defects` очков;
///     если один предаст другого, то первый получит `defect` очков, другой `0`;
///     если оба пойдут на сотрудничество, то они получат по `cooperate` очков.
/// 
/// Модифицированная дилемма заключённого отличается от классической наличием нескольких итераций, 
/// причём программы знают предыдущий выбор соперника. Это позволяет строить, например, 
/// "мстительные" тактики.
pub struct PrisonerDilemma {
    mutual_defects: Score,  // Если оба предадут.
    defect: Score,          // Если один предаст другого.
    cooperate: Score,       // Если оба будут сотрудничать.
}

impl Game for PrisonerDilemma {
    fn round<T1, T2>(
        &self,
        left: &mut T1,
        right: &mut T2,
        iters: u32
    ) -> Result<(Score, Score), GameError>
    where
        T1: Player,
        T2: Player
    {
        // Сообщаем всем участникам количество итераций.
        left.say(format!("{}", iters))
            .map_err(|e| GameError::ErrorLeft(e))?;
        right.say(format!("{}", iters))
            .map_err(|e| GameError::ErrorRight(e))?;
        
        let mut score: (Score, Score) = (0, 0);
        for _ in 0..iters {
            let res = self.iteration(left, right)?;
            score.0 += res.0;
            score.1 += res.1;
        }

        Ok(score)
    }
}

impl PrisonerDilemma {
    pub fn new(mutual_defects: Score, defect: Score, cooperate: Score) -> PrisonerDilemma {
        PrisonerDilemma { mutual_defects, defect, cooperate }
    }

    pub fn default() -> PrisonerDilemma {
        // Стандартная разбалловка
        Self::new(1, 10, 5)
    }

    /// Один выбор игроков с последующим ответом.
    fn iteration<T1, T2>(&self, left: &mut T1, right: &mut T2) -> Result<(Score, Score), GameError>
    where
        T1: Player,
        T2: Player,
    {
        let left_answer = Action::from_str(
            left
                .ask()
                .map_err(|e| GameError::ErrorLeft(e))?
                .as_str())
            .map_err(|e| GameError::ErrorLeft(e))?;
        
        let right_answer = Action::from_str(
            right
                .ask()
                .map_err(|e| GameError::ErrorRight(e))?
                .as_str())
            .map_err(|e| GameError::ErrorRight(e))?;
        
        left.say(
            right_answer
                .to_str()
                .to_string())
            .map_err(|e| GameError::ErrorLeft(e))?;
        right.say(
            left_answer
                .to_str()
                .to_string())
            .map_err(|e| GameError::ErrorRight(e))?;

        Ok(match (&left_answer, &right_answer) {
            (Action::Cooperate, Action::Cooperate) => (self.cooperate, self.cooperate),
            (Action::Cooperate, Action::Betray)    => (0, self.defect),
            (Action::Betray,    Action::Cooperate) => (self.defect, 0),
            (Action::Betray,    Action::Betray)    => (self.mutual_defects, self.mutual_defects),
        })
    }
}

enum Action {
    Cooperate,
    Betray,
}

impl Action {
    fn from_str(s: &str) -> io::Result<Self> {
        match s {
            "COOPERATE" => Ok(Action::Cooperate),
            "BETRAY"    => Ok(Action::Betray),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput, 
                format!("unknown action '{}', expected one of ['COOPERATE', 'BETRAY']", s)
            )),
        }
    }

    fn to_str(&self) -> &'static str {
        match self {
            Action::Cooperate => "COOPERATE",
            Action::Betray    => "BETRAY",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Result;
    use crate::dilemma::PrisonerDilemma;
    use crate::game::{Game, GameError, Player};
 
    #[test]
    fn dilemma_with_two_tit_for_tat_players() {
        // Игроки сыграют в ничью:
        // LEFT | RIGHT |
        // -----+-------|
        //  YES |  NO   |   +10    0
        //   NO |  YES  |     0  +10
        // --------------------------
        //                  +10  +10
        let mut l = TitForTatPlayer::new("COOPERATE");
        let mut r = TitForTatPlayer::new("BETRAY");
        let d_game = PrisonerDilemma::new(1, 10, 5);
        
        let res = d_game.round(&mut l, &mut r, 2);
        
        assert!(res.is_ok(), "unexpected error: {:?}", res.err().unwrap());
        let res = res.unwrap();
        assert_eq!(10, res.0);
        assert_eq!(10, res.1);
    }

    #[test]
    fn dilemma_invalid_output() {
        let mut l = TitForTatPlayer::new("Sth");
        let mut r = TitForTatPlayer::new("BETRAY");
        let d_game = PrisonerDilemma::new(1, 10, 5);

        let res = d_game.round(&mut l, &mut r, 2);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorLeft(_)));
    }
    
    struct TitForTatPlayer {
        started: bool,
        next_choice: String,
    }

    impl TitForTatPlayer {
        fn new(first_choice: &str) -> Self {
            TitForTatPlayer{ started: false, next_choice: first_choice.to_string() }
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
}
