use std::io;

use crate::game::*;
use crate::vprintln;

/// Долларовый аукцион (аукцион двойной цены).
///
/// На торги выставляется приз стоимостью `prize`. Игроки поочерёдно делают ставки
/// (начинает левый). Ключевое правило: оба платят свои последние ставки,
/// но приз получает только победитель (чья ставка выше).
///
/// Игрок может спасовать (ставка 0). Каждая ставка должна быть строго больше
/// последней ставки соперника.
///
/// Ловушка невозвратных затрат: проигрывающий игрок склонен повышать ставку,
/// чтобы отбить вложения, что может привести к ставкам, превышающим приз.
pub struct DollarAuction {
    prize: Score,
}

impl Game for DollarAuction {
    fn round(
        &self,
        left: &mut dyn Player,
        right: &mut dyn Player,
        iters: u32,
    ) -> Result<(Score, Score), GameError> {
        let mut left = GameMediator::new(left);
        let mut right = GameMediator::new(right);

        vprintln!("[init] prize: {}, max rounds: {iters}", self.prize);
        left.initial(self.prize, iters)
            .map_err(GameError::ErrorLeft)?;
        right
            .initial(self.prize, iters)
            .map_err(GameError::ErrorRight)?;

        let mut l_last_bid: Score = 0;
        let mut r_last_bid: Score = 0;

        for i in 0..iters {
            // Ход левого: получает последнюю ставку правого, делает свою
            let l_bid = left.bid(r_last_bid).map_err(GameError::ErrorLeft)?;
            vprintln!("[iter-{i:02}] left bid: {l_bid}");

            if l_bid == 0 {
                vprintln!("[iter-{i:02}] left folds");
                return Ok(self.resolve(l_last_bid, r_last_bid, false));
            }
            l_last_bid = l_bid;

            // Ход правого: получает последнюю ставку левого, делает свою
            let r_bid = right.bid(l_last_bid).map_err(GameError::ErrorRight)?;
            vprintln!("[iter-{i:02}] right bid: {r_bid}");

            if r_bid == 0 {
                vprintln!("[iter-{i:02}] right folds");
                return Ok(self.resolve(l_last_bid, r_last_bid, true));
            }
            r_last_bid = r_bid;
        }

        // Раунды исчерпаны: побеждает с большей ставкой.
        let left_wins = l_last_bid > r_last_bid;
        vprintln!("[result] rounds exhausted, {} wins", if left_wins { "left" } else { "right" });
        Ok(self.resolve(l_last_bid, r_last_bid, left_wins))
    }
}

impl DollarAuction {
    pub fn new(prize: Score) -> DollarAuction {
        DollarAuction { prize }
    }

    pub fn default() -> DollarAuction {
        Self::new(100)
    }

    /// Вычисляет финальный счёт.
    /// `left_wins`: true если левый побеждает, false если правый или никто.
    fn resolve(&self, l_bid: Score, r_bid: Score, left_wins: bool) -> (Score, Score) {
        if l_bid == 0 && r_bid == 0 {
            vprintln!("[result] no bids placed, score: (0, 0)");
            return (0, 0);
        }

        let result = if left_wins {
            (self.prize - l_bid, -r_bid)
        } else {
            (-l_bid, self.prize - r_bid)
        };

        vprintln!("[result] score: {result:?}");
        result
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
    fn initial(&mut self, prize: Score, iters: u32) -> io::Result<()> {
        self.actor.say(format!("{}", prize))?;
        self.actor.say(format!("{}", iters))
    }

    fn bid(&mut self, opponent_last_bid: Score) -> io::Result<Score> {
        self.actor.say(format!("{}", opponent_last_bid))?;

        let bid: Score = self
            .actor
            .ask()?
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        if bid < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("bid must be non-negative, got {}", bid),
            ));
        }

        if bid != 0 && bid <= opponent_last_bid {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "bid must be > opponent's last bid ({}), got {}",
                    opponent_last_bid, bid
                ),
            ));
        }

        Ok(bid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Result;

    struct ScriptedPlayer {
        responses: Vec<String>,
        index: usize,
    }

    impl ScriptedPlayer {
        fn new(responses: Vec<Score>) -> Self {
            ScriptedPlayer {
                responses: responses.iter().map(|r| format!("{}", r)).collect(),
                index: 0,
            }
        }
    }

    impl Player for ScriptedPlayer {
        fn ask(&mut self) -> Result<String> {
            let resp = self.responses[self.index].clone();
            self.index += 1;
            Ok(resp)
        }

        fn say(&mut self, _s: String) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn left_folds_immediately() {
        // Левый сразу пасует, никто не торговался → (0, 0)
        let mut l = ScriptedPlayer::new(vec![0]);
        let mut r = ScriptedPlayer::new(vec![]);
        let game = DollarAuction::new(100);

        let res = game.round(&mut l, &mut r, 10).unwrap();
        assert_eq!(res, (0, 0));
    }

    #[test]
    fn right_folds_after_left_bid() {
        // Левый ставит 1, правый пасует → левый побеждает
        let mut l = ScriptedPlayer::new(vec![1]);
        let mut r = ScriptedPlayer::new(vec![0]);
        let game = DollarAuction::new(100);

        let res = game.round(&mut l, &mut r, 10).unwrap();
        // left wins: P - 1 = 99, right: -0 = 0
        assert_eq!(res, (99, 0));
    }

    #[test]
    fn escalation_then_right_folds() {
        // L=1, R=2, L=3, R пасует
        let mut l = ScriptedPlayer::new(vec![1, 3]);
        let mut r = ScriptedPlayer::new(vec![2, 0]);
        let game = DollarAuction::new(100);

        let res = game.round(&mut l, &mut r, 10).unwrap();
        // left wins with bid 3: P-3=97, right loses with bid 2: -2
        assert_eq!(res, (97, -2));
    }

    #[test]
    fn escalation_then_left_folds() {
        // L=1, R=2, L пасует
        let mut l = ScriptedPlayer::new(vec![1, 0]);
        let mut r = ScriptedPlayer::new(vec![2]);
        let game = DollarAuction::new(100);

        let res = game.round(&mut l, &mut r, 10).unwrap();
        // right wins with bid 2: P-2=98, left loses with bid 1: -1
        assert_eq!(res, (-1, 98));
    }

    #[test]
    fn bids_exceed_prize() {
        // Ставки превышают приз — ловушка эскалации
        // L=50, R=60, L=70, R пасует
        let mut l = ScriptedPlayer::new(vec![50, 70]);
        let mut r = ScriptedPlayer::new(vec![60, 0]);
        let game = DollarAuction::new(100);

        let res = game.round(&mut l, &mut r, 10).unwrap();
        // left wins with bid 70: P-70=30, right loses with bid 60: -60
        assert_eq!(res, (30, -60));
    }

    #[test]
    fn bids_way_exceed_prize() {
        // Обе ставки > приза
        // L=80, R=90, L=110, R пасует
        let mut l = ScriptedPlayer::new(vec![80, 110]);
        let mut r = ScriptedPlayer::new(vec![90, 0]);
        let game = DollarAuction::new(100);

        let res = game.round(&mut l, &mut r, 10).unwrap();
        // left wins with bid 110: P-110 = -10, right loses with bid 90: -90
        assert_eq!(res, (-10, -90));
    }

    #[test]
    fn iters_exhausted() {
        // 2 раунда, никто не пасует
        // Round 0: L=1, R=2
        // Round 1: L=3, R=4
        // Раунды исчерпаны, правый побеждает (последний ходил)
        let mut l = ScriptedPlayer::new(vec![1, 3]);
        let mut r = ScriptedPlayer::new(vec![2, 4]);
        let game = DollarAuction::new(100);

        let res = game.round(&mut l, &mut r, 2).unwrap();
        // right wins: P-4=96, left loses: -3
        assert_eq!(res, (-3, 96));
    }

    #[test]
    fn invalid_bid_not_higher() {
        // Левый ставит 5, правый ставит 3 (не больше 5) → ошибка
        let mut l = ScriptedPlayer::new(vec![5]);
        let mut r = ScriptedPlayer::new(vec![3]);
        let game = DollarAuction::new(100);

        let res = game.round(&mut l, &mut r, 10);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorRight(_)));
    }

    #[test]
    fn invalid_bid_equal() {
        // Левый ставит 5, правый ставит 5 (не строго больше) → ошибка
        let mut l = ScriptedPlayer::new(vec![5]);
        let mut r = ScriptedPlayer::new(vec![5]);
        let game = DollarAuction::new(100);

        let res = game.round(&mut l, &mut r, 10);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorRight(_)));
    }

    struct InvalidPlayer;

    impl Player for InvalidPlayer {
        fn ask(&mut self) -> Result<String> {
            Ok("xyz".to_string())
        }

        fn say(&mut self, _s: String) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn non_numeric_bid() {
        let mut l = InvalidPlayer;
        let mut r = ScriptedPlayer::new(vec![]);
        let game = DollarAuction::new(100);

        let res = game.round(&mut l, &mut r, 10);
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), GameError::ErrorLeft(_)));
    }
}
