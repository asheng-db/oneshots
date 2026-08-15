//! Terminal rendering: piece letters, trap markers, last-move highlight,
//! piece counts, and optional ANSI color.

use crate::board::{Cell, Color, TRAPS};
use crate::game::Game;

/// ANSI color wrap; Gold = yellow (33), Silver = cyan (36).
fn colored(color: Color, glyph: &str, use_color: bool) -> String {
    if !use_color {
        return glyph.to_string();
    }
    let code = match color {
        Color::Gold => "33",
        Color::Silver => "36",
    };
    format!("\x1b[{}m{}\x1b[0m", code, glyph)
}

/// Render the board. Highlights `game.last_step` (most recent step target).
pub fn render(game: &Game, use_color: bool) -> String {
    let b = &game.board;
    let last = game.last_step;
    let mut s = String::new();
    s.push_str("     a  b  c  d  e  f  g  h\n");
    for r in (0..8).rev() {
        s.push_str(&format!("{}   ", r + 1));
        for c in 0..8 {
            let cell = b.at(r, c);
            let is_trap = TRAPS.iter().any(|t| t[0] == r && t[1] == c);
            let is_last = last == Some((r, c));
            let glyph = match cell {
                Cell::Empty => {
                    if is_trap {
                        "💥".to_string()
                    } else {
                        "·".to_string()
                    }
                }
                Cell::Piece(p) => colored(p.color, &p.letter().to_string(), use_color),
            };
            // Highlight the last-move square in bold reverse video.
            let mut g = if is_last {
                format!("\x1b[1;7m{}\x1b[0m", glyph)
            } else {
                glyph
            };
            if is_trap && matches!(cell, Cell::Piece(_)) {
                g += "💥";
            }
            s.push_str(&format!("{:<3}", g));
        }
        s.push('\n');
    }
    // Piece counts footer.
    let gold = b.count_pieces(Color::Gold);
    let silver = b.count_pieces(Color::Silver);
    let gold_r = b.count_rabbits(Color::Gold);
    let silver_r = b.count_rabbits(Color::Silver);
    s.push_str(&format!(
        "Gold: {} pieces ({} rabbits)   Silver: {} pieces ({} rabbits)\n",
        gold, gold_r, silver, silver_r
    ));
    s
}
