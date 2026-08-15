mod board;
mod bot;
mod game;
mod render;

use std::env;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use game::Game;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut seed: Option<u64> = None;
    let mut no_color = false;
    let mut replay = false;
    let mut fast = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => { seed = args.get(i + 1).map(|s| s.parse().ok()).flatten(); i += 1; }
            "--no-color" => no_color = true,
            "--replay" => replay = true,
            "--fast" => fast = true,
            "--help" | "-h" => {
                println!("Arimaa simulator");
                println!("  --seed N     fixed RNG seed (deterministic)");
                println!("  --no-color   disable ANSI colors");
                println!("  --replay     replay from the recorded move log");
                println!("  --fast       no delay between turns");
                return;
            }
            _ => {}
        }
        i += 1;
    }

    // Seed: CLI > ARIMAA_SEED env > time.
    let seed = seed.or_else(|| env::var("ARIMAA_SEED").ok().and_then(|s| s.parse().ok())).unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(42)
            ^ 0xDEADBEEFCAFEF00D
    });

    let mut game = Game::new(seed);

    println!("=== Arimaa: Gold vs Silver (bots) ===");
    println!("Trap squares (💥): c3, f3, c6, f6.");
    println!("Gold (E/M/H/D/C/R) at bottom, Silver (e/m/h/d/c/r) at top.");
    println!("Gold wins by reaching row 8, Silver by reaching row 1.\n");
    println!("Turn {} - {} to move", game.turn_number, game.turn.name());
    println!("{}", render::render(&game, !no_color));

    game.push_history();

    loop {
        let color = game.turn;
        game.play_turn(color, bot::choose_action);
        game.push_history();
        game.turn = game.turn.other();
        game.turn_number += 1;

        // --- End conditions after the turn ---
        // Goal
        if game.board.goal_for(color) {
            println!("\nTurn {} - {} moves:", game.turn_number, game.turn.name());
            println!("{}", render::render(&game, !no_color));
            println!("\n🏆 {} wins by GOAL!", color.name());
            break;
        }
        // Elimination
        if game.board.count_rabbits(color.other()) == 0 {
            println!("\nTurn {} - {} moves:", game.turn_number, game.turn.name());
            println!("{}", render::render(&game, !no_color));
            println!("\n🏆 {} wins by ELIMINATION (all {} rabbits captured)!",
                color.name(), color.other().name());
            break;
        }
        // Immobilization: opponent has no legal step.
        if game.legal_steps(game.turn).is_empty() {
            println!("\nTurn {} - {} moves:", game.turn_number, game.turn.name());
            println!("{}", render::render(&game, !no_color));
            println!("\n🏆 {} wins by IMMOBILIZATION ({} has no legal moves)!",
                color.name(), game.turn.name());
            break;
        }
        // Repetition: same position 3 times -> draw.
        if game.repeated(3) {
            println!("\nTurn {} - {}", game.turn_number, game.turn.name());
            println!("{}", render::render(&game, !no_color));
            println!("\n🤝 Draw by repetition (3-fold).");
            break;
        }

        println!("\nTurn {} - {} to move", game.turn_number, game.turn.name());
        println!("{}", render::render(&game, !no_color));
        if !fast && env::var("ARIMAA_FAST").is_err() {
            sleep(Duration::from_secs(1));
        }
    }

    // Log the move list.
    println!("\nMove log ({} entries):", game.move_log.len());
    for m in &game.move_log {
        println!("  {}", m.describe(game.turn, game.turn_number));
    }

    // Replay mode: re-apply the recorded actions to a fresh game, showing the
    // board after each recorded action.
    if replay {
        println!("\n=== Replay ===");
        let mut rg = Game::new(seed);
        println!("Turn 1 - {} to move", rg.turn.name());
        println!("{}", render::render(&rg, !no_color));
        for m in &game.move_log {
            rg.apply_action(&m.action);
            println!("  {}", m.describe(rg.turn, rg.turn_number));
            println!("{}", render::render(&rg, !no_color));
            if !fast && env::var("ARIMAA_FAST").is_err() {
                sleep(Duration::from_secs(1));
            }
        }
    }

    println!("Game finished after {} turns.", game.turn_number);
}
