//! Game state, legal moves, turn engine, and position history.

use rand::prelude::*;
use rand::SeedableRng;
use rand::rngs::SmallRng;

use crate::board::{Board, Cell, Color, PType, neighbors};

/// One full 1-2 step action a player can take within a turn.
#[derive(Clone, Debug)]
pub enum Action {
    /// Move a piece from (fr,fc) to an empty (tr,tc). Costs 1 step.
    Step { fr: usize, fc: usize, tr: usize, tc: usize },
    /// Push/pull a weaker enemy piece. Costs 2 steps.
    Dislodge(Dislodge),
}

/// A validated 2-step dislodge; board_after is the resolved board.
#[derive(Clone, Debug)]
pub struct Dislodge {
    pub board_after: Board,
}

/// A single recorded move for logging/replay.
#[derive(Clone, Debug)]
pub struct MoveRec {
    pub action: Action,
}

impl MoveRec {
    /// Human-readable description like "G E d3->d4" or "G M d3xr4 (dislodge)".
    pub fn describe(&self, color: Color, turn: usize) -> String {
        let c = match color {
            Color::Gold => "G",
            Color::Silver => "S",
        };
        match &self.action {
            Action::Step { fr, fc, tr, tc } => {
                format!("{} T{}: step {:?}->{:?}", c, turn, (fr, fc), (tr, tc))
            }
            Action::Dislodge(_) => {
                format!("{} T{}: dislodge", c, turn)
            }
        }
    }
}

pub struct Game {
    pub board: Board,
    pub turn: Color,
    pub turn_number: usize,
    pub rng: SmallRng,
    pub move_log: Vec<MoveRec>,
    /// Target square of the most recent step, for board highlight.
    pub last_step: Option<(usize, usize)>,
    /// Position history (canonical board key) for repetition detection.
    history: Vec<String>,
}

impl Game {
    pub fn new(seed: u64) -> Game {
        let mut board = Board::new();
        board.setup();
        Game {
            board,
            turn: Color::Gold,
            turn_number: 1,
            rng: seed_rng(seed),
            move_log: Vec::new(),
            last_step: None,
            history: Vec::new(),
        }
    }

    /// Canonical board key: letters row-major, gold uppercase / silver lowercase,
    /// empty cells as '.'. Independent of board orientation.
    pub fn board_key(&self) -> String {
        let mut s = String::with_capacity(64);
        for r in 0..8 {
            for c in 0..8 {
                match self.board.at(r, c) {
                    Cell::Empty => s.push('.'),
                    Cell::Piece(p) => s.push(p.letter()),
                }
            }
        }
        s
    }

    /// Record current position into history for repetition checks.
    pub fn push_history(&mut self) {
        self.history.push(self.board_key());
    }

    /// True if the same position has occurred `n` times (3-fold repetition).
    pub fn repeated(&self, n: usize) -> bool {
        let key = self.board_key();
        self.history.iter().filter(|k| k.as_str() == key).count() >= n
    }

    // All legal single steps for `color`. Ported from the original engine.
    pub fn legal_steps(&self, color: Color) -> Vec<(usize, usize, usize, usize)> {
        let mut steps = Vec::new();
        for r in 0..8 {
            for c in 0..8 {
                let p = match self.board.piece_at(r, c) {
                    Some(p) if p.color == color => p,
                    _ => continue,
                };
                if self.board.is_frozen(r, c) {
                    continue;
                }
                for (nr, nc) in neighbors(r, c) {
                    if self.board.at(nr, nc) != Cell::Empty {
                        continue;
                    }
                    let backward = match color {
                        Color::Gold => nr < r,
                        Color::Silver => nr > r,
                    };
                    if p.ptype == PType::Rabbit && backward {
                        continue;
                    }
                    steps.push((r, c, nr, nc));
                }
            }
        }
        steps
    }

    // All legal dislodge actions for `color`. Ported from the original engine.
    pub fn legal_dislodge(&self, color: Color) -> Vec<Dislodge> {
        let mut out = Vec::new();
        for mr in 0..8 {
            for mc in 0..8 {
                let mover = match self.board.piece_at(mr, mc) {
                    Some(p) if p.color == color => p,
                    _ => continue,
                };
                if self.board.is_frozen(mr, mc) {
                    continue;
                }
                for (er, ec) in neighbors(mr, mc) {
                    let enemy = match self.board.piece_at(er, ec) {
                        Some(p) if p.color == color.other() => p,
                        _ => continue,
                    };
                    if enemy.strength() >= mover.strength() {
                        continue;
                    }
                    // --- Pull ---
                    for (nr, nc) in neighbors(mr, mc) {
                        if self.board.at(nr, nc) != Cell::Empty {
                            continue;
                        }
                        let mut b = self.board.clone();
                        b.set(mr, mc, Cell::Empty);
                        b.set(er, ec, Cell::Empty);
                        b.set(nr, nc, Cell::Piece(mover));
                        b.set(mr, mc, Cell::Piece(enemy));
                        b.apply_traps();
                        out.push(Dislodge { board_after: b });
                    }
                    // --- Push ---
                    for (nr, nc) in neighbors(er, ec) {
                        if self.board.at(nr, nc) != Cell::Empty {
                            continue;
                        }
                        let mut b = self.board.clone();
                        b.set(er, ec, Cell::Empty);
                        b.set(mr, mc, Cell::Empty);
                        b.set(nr, nc, Cell::Piece(enemy));
                        b.set(er, ec, Cell::Piece(mover));
                        b.apply_traps();
                        out.push(Dislodge { board_after: b });
                    }
                }
            }
        }
        out
    }

    /// All legal actions (steps + at most one dislodge) for `color`.
    /// `dislodged` true means no further dislodge actions are included.
    pub fn legal_actions(&self, color: Color, dislodged: bool) -> Vec<Action> {
        let mut actions: Vec<Action> = Vec::new();
        for &(fr, fc, tr, tc) in &self.legal_steps(color) {
            actions.push(Action::Step { fr, fc, tr, tc });
        }
        if !dislodged {
            for d in self.legal_dislodge(color) {
                actions.push(Action::Dislodge(d));
            }
        }
        actions
    }

    /// Apply one action to the board, returning the steps it consumed.
    /// Records the last step target for highlight.
    pub fn apply_action(&mut self, action: &Action) -> usize {
        match action {
            Action::Step { fr, fc, tr, tc } => {
                let p = self.board.piece_at(*fr, *fc).unwrap();
                self.board.set(*fr, *fc, Cell::Empty);
                self.board.set(*tr, *tc, Cell::Piece(p));
                self.board.apply_traps();
                self.last_step = Some((*tr, *tc));
                1
            }
            Action::Dislodge(d) => {
                self.board = d.board_after.clone();
                2
            }
        }
    }

    /// Play one full turn for `color` using the provided bot function.
    /// The bot picks actions until the step budget (1-4) is exhausted or no
    /// legal actions remain. Returns the number of steps used.
    pub fn play_turn<F>(&mut self, color: Color, bot: F) -> usize
    where
        F: Fn(&mut Game, Color, bool) -> Action,
    {
        let steps_left = 1 + self.rng.random_range(0..=3) as usize;
        let mut steps_used = 0usize;
        let mut dislodged = false;

        while steps_used < steps_left {
            let actions = self.legal_actions(color, dislodged);
            if actions.is_empty() {
                break;
            }
            let action = bot(self, color, dislodged);
            self.move_log.push(MoveRec {
                action: action.clone(),
            });
            steps_used += self.apply_action(&action);
            if matches!(action, Action::Dislodge(_)) {
                dislodged = true;
            }
        }
        steps_used
    }
}

/// Derive a 32-byte seed for SmallRng from a u64.
pub fn seed_rng(seed: u64) -> SmallRng {
    let mut s = [0u8; 32];
    s[0..8].copy_from_slice(&seed.to_le_bytes());
    s[8..16].copy_from_slice(&(!seed).to_le_bytes());
    let x = seed.wrapping_mul(0x9E3779B97F4A7C15).rotate_left(32);
    s[16..24].copy_from_slice(&x.to_le_bytes());
    s[24..32].copy_from_slice(&(!x).to_le_bytes());
    SmallRng::from_seed(s)
}
