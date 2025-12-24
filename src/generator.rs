use crate::board::Board;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

pub struct Generator;

impl Generator {
    /// Generate a random solved sudoku board
    pub fn random_solved_board() -> Board {
        let rng = StdRng::from_entropy();
        Self::random_solved_board_seeded(rng)
    }

    /// Generate a random solved sudoku board with a specific seed
    pub fn random_solved_board_seeded(mut rng: StdRng) -> Board {
        let mut board = Board::new();
        let mut values: Vec<u8> = (1..=9).collect();

        // Fill first row randomly
        values.shuffle(&mut rng);
        for (col, &value) in values.iter().enumerate() {
            board.set_cell(0, col, value).ok();
        }

        // Fill remaining cells using backtracking with randomization
        Self::fill_board_random(&mut board, &mut rng);
        board
    }

    /// Recursively fill board with random valid placements
    fn fill_board_random(board: &mut Board, rng: &mut StdRng) -> bool {
        let unsolved = board.get_unsolved();
        if unsolved.is_empty() {
            return true;
        }

        let (row, col) = unsolved[0];
        let cell = board.get_cell(row, col).unwrap();
        let mut candidates: Vec<u8> = cell.possible.iter().copied().collect();
        candidates.shuffle(rng);

        for value in candidates {
            let mut test_board = board.deep_clone();
            test_board.set_cell(row, col, value).ok();

            if test_board.is_valid() && Self::fill_board_random(&mut test_board, rng) {
                *board = test_board;
                return true;
            }
        }

        false
    }

    /// Generate a puzzle from a solved board by removing clues
    pub fn create_puzzle(solved_board: &Board) -> (Board, String) {
        let mut puzzle = solved_board.deep_clone();
        let mut rng = StdRng::from_entropy();
        let solution_id = Self::board_to_id(solved_board);

        // Greedy approach: randomly remove cells while puzzle is still solvable
        let mut puzzle_str = puzzle.to_string();
        let mut puzzle_chars: Vec<char> = puzzle_str.chars().collect();
        let mut removed_positions: Vec<usize> = (0..81).collect();
        removed_positions.shuffle(&mut rng);

        for idx in removed_positions {
            if puzzle_chars[idx] != '0' {
                // Try removing this cell
                let original = puzzle_chars[idx];
                puzzle_chars[idx] = '0';
                puzzle_str = puzzle_chars.iter().collect();

                // Check if puzzle is still solvable
                if let Ok(test_board) = Board::from_string(&puzzle_str) {
                    let result = crate::solver::Solver::solve(&test_board);
                    // Keep the removal if it's still solvable
                    if result.status == crate::solver::SolveStatus::Solved {
                        // Continue with next removal
                        continue;
                    }
                }

                // Restore if removal made puzzle unsolvable
                puzzle_chars[idx] = original;
            }
        }

        puzzle_str = puzzle_chars.iter().collect();
        puzzle = Board::from_string(&puzzle_str).unwrap_or_else(|_| solved_board.deep_clone());

        (puzzle, solution_id)
    }

    /// Convert board to unique ID
    pub fn board_to_id(board: &Board) -> String {
        let board_str = board.to_string();
        // Simple ID generation using hash of normalized board
        let normalized = Self::normalize_board(&board_str);
        format!("{:x}", simple_hash(&normalized))
    }

    /// Normalize board to canonical form (minimal lexicographic)
    fn normalize_board(s: &str) -> String {
        s.to_string() // Simplified - full implementation would find minimal rotation/reflection
    }

    /// Count filled cells in a board
    pub fn hint_count(board: &Board) -> usize {
        81 - board.unsolved_count()
    }
}

/// Simple hash function for board IDs
fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for c in s.chars() {
        hash = hash.wrapping_mul(33).wrapping_add(c as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use crate::generator::{Board, Generator, SeedableRng, StdRng};
    use crate::solver::{SolveStatus, Solver};

    #[test]
    fn test_random_solved_board() {
        let board = Generator::random_solved_board();
        let result = Solver::solve(&board);
        assert_eq!(result.status, SolveStatus::Solved);
    }

    #[test]
    fn test_seeded_generation() {
        let rng1 = StdRng::seed_from_u64(42);
        let board1 = Generator::random_solved_board_seeded(rng1);

        let rng2 = StdRng::seed_from_u64(42);
        let board2 = Generator::random_solved_board_seeded(rng2);

        assert_eq!(board1.to_string(), board2.to_string());
    }

    #[test]
    fn test_create_puzzle() {
        let solved = Generator::random_solved_board();
        let (puzzle, _id) = Generator::create_puzzle(&solved);

        // Verify puzzle is solvable
        let result = Solver::solve(&puzzle);
        assert_eq!(result.status, SolveStatus::Solved);

        // Verify puzzle has fewer hints than solved board
        assert!(Generator::hint_count(&puzzle) < Generator::hint_count(&solved));
    }

    #[test]
    fn test_board_to_id() {
        let board = Board::from_string(
            "003020600900305001001806400008102900700000008006708200002609500800203006005010300",
        )
        .unwrap();
        let id = Generator::board_to_id(&board);
        assert!(!id.is_empty());
    }
}
