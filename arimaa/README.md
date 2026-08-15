# Arimaa Simulator

A Rust program that simulates full games of
[Arimaa](https://en.wikipedia.org/wiki/Arimaa) between two heuristic bots. The
game state is rendered to the terminal each turn using piece letters, with a
one-second pause after every turn (skippable).

Author: **Deepseek v4 Flash**

## Overview

Arimaa is a two-player abstract strategy board game designed (by Omar Syed,
2002) to be easy for humans but hard for computers. This project implements the
full rules and lets two bots play until a game ends, with deterministic replay
and logging.

## Rules implemented

- **Board**: 8×8 with four trap squares at `c3`, `f3`, `c6`, `f6`.
- **Pieces** (strongest → weakest): Elephant, Camel, Horse, Dog, Cat, Rabbit.
- **Material** per side: 1 Elephant, 1 Camel, 2 Horses, 2 Dogs, 2 Cats,
  8 Rabbits (16 total).
- **Setup**: Fixed placement matching Wikipedia's Diagram 1 — Gold on ranks
  1–2, Silver on ranks 7–8.
- **Turns**: 1–4 steps per turn, distributed across pieces, with at most one
  push or pull (dislodge) per turn. Rabbits may not step backward.
- **Freeze**: a piece orthogonally adjacent to a stronger enemy piece is frozen
  (can't move) unless it has an adjacent friendly piece.
- **Capture**: a piece on a trap square is removed unless a friendly piece is
  orthogonally adjacent (checked after each step).
- **Win conditions**:
  - *Goal* — a rabbit reaches the opponent's home rank.
  - *Elimination* — all of the opponent's rabbits are captured.
  - *Immobilization* — the opponent has no legal step at the start of its turn.
- **Repetition**: if the same position occurs three times (3-fold repetition),
  the game is a draw.

## Building & running

Requires a Rust toolchain (`rustc` / `cargo`).

```sh
cd arimaa
cargo run --release
```

Each turn prints the board (piece letters, 💥 = trap square) then pauses for
**1 second**. Gold pieces are uppercase and shown in yellow; Silver pieces are
lowercase and shown in cyan. The most recent step square is highlighted in bold
reverse video.

### CLI flags

| Flag | Effect |
|------|--------|
| `--seed N` | Fixed RNG seed → deterministic game |
| `--no-color` | Disable ANSI colors |
| `--replay` | Re-apply the recorded move log and show the board after each move |
| `--fast` | Skip the 1-second delay between turns |

The seed can also be set via the `ARIMAA_FAST`/`ARIMAA_SEED` environment
variables (`ARIMAA_FAST=1` skips the delay, `ARIMAA_SEED=42` fixes the seed).

## Project layout

```
arimaa/
├── Cargo.toml        # package manifest (dep: rand)
└── src/
    ├── board.rs      # board model: pieces, cells, traps, freeze/capture/goal
    ├── bot.rs        # heuristic move selection with random noise
    ├── game.rs       # game state, legal moves, turn engine, RNG seeding
    ├── render.rs     # terminal rendering (letters, colors, highlights, counts)
    └── main.rs       # CLI parsing, game loop, logging, replay
```

## Design notes

- `Board::cells` is an 8×8 grid of `Cell` (`Empty` / `Piece`).
- `legal_steps` enumerates validated single steps; `legal_dislodge` builds a
  full post-dislodge board copy (with traps applied) so legality is checked
  against the resulting position.
- Turn logic enforces the "at most one dislodge per turn" rule.
- Win conditions are checked after each turn: goal, elimination,
  immobilization, then 3-fold repetition (draw).
- The RNG (`rand::rngs::SmallRng`) is seeded from a single `u64`, so a fixed
  seed reproduces an identical game exactly.
- The bot scores candidate moves (avoiding unsupported trap steps, pushing
  rabbits toward the goal, capturing enemy pieces) plus a small random noise
  term to keep play varied.

## Testing

```sh
cargo test
```

Unit tests cover trap squares, piece letters, and initial setup counts.
