//! Bots: move selection heuristics with randomness (noise).

use rand::prelude::*;

use crate::board::{Color, PType, TRAPS};
use crate::game::{Action, Game};

/// Pick an action for `color` given the current game.
///
/// Heuristic: prefer captures (dislodge) and rabbit progress toward the goal,
/// avoid moves that step a piece onto a trap without support. A random noise
/// term keeps play varied and non-deterministic (unless a fixed seed is used).
pub fn choose_action(game: &mut Game, color: Color, _dislodged: bool) -> Action {
    let actions = game.legal_actions(color, _dislodged);
    if actions.is_empty() {
        // No legal action; caller should have stopped. Return a dummy step.
        return Action::Step { fr: 0, fc: 0, tr: 0, tc: 0 };
    }

    let mut best_score = f64::NEG_INFINITY;
    let mut best: Option<Action> = None;
    for a in &actions {
        let score = score_action(game, color, a);
        // Add noise: up to ±1.0 so near-equal moves are chosen randomly.
        let noise = game.rng.random_range(-1.0..=1.0);
        let total = score + noise;
        if best.is_none() || total > best_score {
            best_score = total;
            best = Some(a.clone());
        }
    }
    best.unwrap()
}

/// Score one action. Higher = better for `color`.
fn score_action(game: &Game, color: Color, action: &Action) -> f64 {
    let b = &game.board;
    let mut s = 0.0;

    match action {
        Action::Step { fr, fc, tr, tc } => {
            // Avoid stepping onto a trap with no friendly support.
            if TRAPS.iter().any(|t| t[0] == *tr && t[1] == *tc) {
                let supported = crate::board::neighbors(*tr, *tc)
                    .iter()
                    .any(|&(nr, nc)| {
                        matches!(b.piece_at(nr, nc), Some(q) if q.color == color)
                    });
                if !supported {
                    s -= 10.0;
                }
            }
            // Rabbit progress toward the goal rank.
            if let Some(p) = b.piece_at(*fr, *fc) {
                if p.ptype == PType::Rabbit {
                    let toward = match color {
                        Color::Gold => *tr as i32 - *fr as i32,
                        Color::Silver => *fr as i32 - *tr as i32,
                    };
                    s += toward as f64 * 2.0;
                }
            }
            s += 0.5; // small bias toward doing anything
        }
        Action::Dislodge(d) => {
            // Capturing an enemy piece is strongly good; capturing a rabbit is
            // even better (elimination win).
            let before = game.board.count_pieces(color.other());
            let after = d.board_after.count_pieces(color.other());
            let captured = before - after;
            s += captured as f64 * 5.0;
            s += 3.0; // dislodge action bias (uses 2 steps)
        }
    }
    s
}
