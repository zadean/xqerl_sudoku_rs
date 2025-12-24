mod board;
mod generator;
mod solver;
mod svg;

use wasm_bindgen::prelude::*;
use serde_json;

pub use board::Board;
pub use generator::Generator;
pub use solver::{Solver, SolveStatus};

// Generate a random puzzle and return as JSON string
#[wasm_bindgen]
pub fn generate_puzzle() -> Result<String, String> {
    // Generate solved board
    let solved_board = Generator::random_solved_board();

    // Create puzzle from solved board
    let (puzzle, _) = Generator::create_puzzle(&solved_board);

    // Solve puzzle to get difficulty and hint count
    let result = solver::Solver::solve(&puzzle);

    // Validate that the solved board matches
    if result.board.to_string() != solved_board.to_string() {
        return Err("Puzzle validation failed".to_string());
    }

    let difficulty = result.score.difficulty();
    let hint_count = Generator::hint_count(&puzzle);

    let puzzle_data = serde_json::json!({
        "puzzle_hints": puzzle.to_string(),
        "solved_board": solved_board.to_string(),
        "difficulty": difficulty,
        "hint_count": hint_count,
    });

    Ok(puzzle_data.to_string())
}

// Solve an existing puzzle and return as JSON string
#[wasm_bindgen]
pub fn solve_puzzle(puzzle_str: &str) -> Result<String, String> {
    let board = Board::from_string(puzzle_str).map_err(|e| e.to_string())?;
    let result = Solver::solve(&board);

    let solve_result = serde_json::json!({
        "solved_board": result.board.to_string(),
        "difficulty": result.score.difficulty(),
        "status": format!("{:?}", result.status),
        "weight": result.score.total_weight(),
    });

    Ok(solve_result.to_string())
}

// Validate a puzzle string
#[wasm_bindgen]
pub fn board_from_string(puzzle_str: &str) -> Result<String, String> {
    Board::from_string(puzzle_str)
        .map(|_| "Valid".to_string())
        .map_err(|e| e.to_string())
}

// Render puzzle as SVG
#[wasm_bindgen]
pub fn render_svg(puzzle_str: &str) -> Result<String, String> {
    svg::SvgRenderer::render_puzzle(puzzle_str).map_err(|e| e.to_string())
}

// Get hint count for a puzzle
#[wasm_bindgen]
pub fn get_hint_count(puzzle_str: &str) -> u32 {
    Generator::hint_count(&Board::from_string(puzzle_str).unwrap_or_default()) as u32
}
