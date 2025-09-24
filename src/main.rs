mod engine;

use pleco::{Board, Player};
use std::fs;
use rand::rng;
use rand::distr::{Distribution, Uniform};

fn parse_csv_line(line: &str) -> Option<(String, String, Vec<String>)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = vec![];
    let mut in_quotes = false;
    let mut current = String::new();
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' {
            in_quotes = !in_quotes;
        } else if b == b',' && !in_quotes {
            let trimmed_part = current.trim_matches('"').trim().to_string();
            parts.push(trimmed_part);
            current.clear();
        } else {
            current.push(b as char);
        }
        i += 1;
    }
    let trimmed_part = current.trim_matches('"').trim().to_string();
    parts.push(trimmed_part);
    if parts.len() >= 3 {
        let eco = parts[0].clone();
        let name = parts[1].clone();
        let moves_str = parts[2].clone();
        let moves: Vec<String> = moves_str.split_whitespace()
            .filter(|s| {
                if let Some(first) = s.chars().next() {
                    !(first.is_digit(10) && s.ends_with('.'))
                } else {
                    true
                }
            })
            .map(|s| s.to_string())
            .collect();
        Some((eco, name, moves))
    } else {
        None
    }
}

fn parse_san(board: &Board, san: &str) -> Result<pleco::BitMove, String> {
    engine::san_to_bitmove(board, san)
}

fn main() {
    let mut board = Board::start_pos();
    println!("Starting game:\n{:?}", board);

    // Read openings from CSV
    let contents = fs::read_to_string("openings_sheet.csv")
        .expect("Failed to read openings_sheet.csv");
    let lines: Vec<&str> = contents.lines().collect();
    let openings: Vec<(String, String, Vec<String>)> = lines.iter()
        .skip(1) // Skip header
        .filter_map(|line| parse_csv_line(line))
        .collect();

    if !openings.is_empty() {
        let mut rng = rng();
        let dist = Uniform::new(0usize, openings.len()).unwrap();
        let selected_idx = dist.sample(&mut rng);
        let (eco, name, moves) = &openings[selected_idx];
        println!("Selected opening: {} - {}", eco, name);

        // Apply opening moves using parse_san
        for san_move in moves.iter() {
            match parse_san(&board, san_move.as_str()) {
                Ok(mv) => {
                    board.apply_move(mv);
                    println!("Applied move: {} - Board: {:?}", san_move, board);
                }
                Err(e) => {
                    println!("Invalid move in opening: {} - Error: {}", san_move, e);
                    break;
                }
            }
        }
        println!("Opening complete. Board after opening:\n{:?}", board);
    }

    while !board.checkmate() && !board.stalemate() {
        let best_move = engine::get_best_move(&mut board, engine::MAX_DEPTH);
        if best_move.is_null() {
            break;
        }
        println!("Best move: {:?} (turn: {:?})", best_move, board.turn());
        board.apply_move(best_move);
        println!("Board after move:\n{:?}", board);
    }

    if board.checkmate() {
        let winner = if board.turn() == Player::White { "Black" } else { "White" };
        println!("Checkmate! {} wins.", winner);
    } else if board.stalemate() {
        println!("Stalemate! It's a draw.");
    } else {
        println!("Game ended.");
    }
    println!("Final board:\n{:?}", board);
}