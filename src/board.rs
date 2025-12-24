use std::collections::HashSet;
use std::fmt;

/// Represents a single cell in a sudoku board
#[derive(Clone, Debug)]
pub struct Cell {
    pub value: u8,             // 0 if empty, 1-9 if set
    pub possible: HashSet<u8>, // Candidate values
}

impl Cell {
    pub fn new() -> Self {
        Self {
            value: 0,
            possible: (1..=9).collect(),
        }
    }

    pub fn set_value(&mut self, value: u8) {
        self.value = value;
        self.possible.clear();
    }

    pub fn remove_possible(&mut self, candidate: u8) {
        self.possible.remove(&candidate);
    }

    pub fn is_solved(&self) -> bool {
        self.value > 0
    }

    pub fn num_candidates(&self) -> usize {
        self.possible.len()
    }
}

/// Represents a 9x9 sudoku board
#[derive(Clone, Debug)]
pub struct Board {
    cells: [[Cell; 9]; 9],
    unsolved_count: usize,
}

impl Board {
    pub fn new() -> Self {
        Self {
            cells: std::array::from_fn(|_| std::array::from_fn(|_| Cell::new())),
            unsolved_count: 81,
        }
    }

    /// Create board from an 81-character string (0 for empty, 1-9 for values)
    pub fn from_string(s: &str) -> Result<Self, String> {
        if s.len() != 81 {
            return Err("Board string must be exactly 81 characters".to_string());
        }

        let mut board = Self::new();
        let chars: Vec<char> = s.chars().collect();

        for (idx, ch) in chars.iter().enumerate() {
            let value: u32 = ch.to_digit(10).ok_or("Invalid character in board string")?;
            let value = value as u8;

            if value > 9 {
                return Err("Values must be 0-9".to_string());
            }

            let (row, col) = (idx / 9, idx % 9);
            if value > 0 {
                board.set_cell(row, col, value)?;
            }
        }

        Ok(board)
    }

    /// Convert board to 81-character string
    pub fn to_string(&self) -> String {
        let mut result = String::with_capacity(81);
        for row in 0..9 {
            for col in 0..9 {
                result.push(char::from_digit(self.cells[row][col].value as u32, 10).unwrap());
            }
        }
        result
    }

    /// Set a cell value and update visibility constraints
    pub fn set_cell(&mut self, row: usize, col: usize, value: u8) -> Result<(), String> {
        if row >= 9 || col >= 9 || value > 9 {
            return Err("Invalid row, col, or value".to_string());
        }

        if value == 0 {
            return Err("Use clear_cell to unset a cell".to_string());
        }

        if self.cells[row][col].value == 0 {
            self.unsolved_count -= 1;
        }

        self.cells[row][col].set_value(value);

        // Remove from visible cells' possibles
        let visible = Self::get_visible_cells_indices(row, col);
        for (r, c) in visible {
            self.cells[r][c].remove_possible(value);
        }

        Ok(())
    }

    /// Clear a cell value (set it back to empty)
    #[allow(dead_code)]
    pub fn clear_cell(&mut self, row: usize, col: usize) -> Result<(), String> {
        if row >= 9 || col >= 9 {
            return Err("Invalid row or col".to_string());
        }

        if self.cells[row][col].value > 0 {
            self.unsolved_count += 1;
            self.cells[row][col].value = 0;
            // Reset possible values
            self.cells[row][col].possible = (1..=9).collect();
        }

        Ok(())
    }

    /// Get a cell reference
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&Cell> {
        if row >= 9 || col >= 9 {
            None
        } else {
            Some(&self.cells[row][col])
        }
    }

    /// Get mutable cell reference
    pub fn get_cell_mut(&mut self, row: usize, col: usize) -> Option<&mut Cell> {
        if row >= 9 || col >= 9 {
            None
        } else {
            Some(&mut self.cells[row][col])
        }
    }

    /// Get cell reference by linear index (0-80)
    pub fn get_cell_by_idx(&self, idx: usize) -> Option<&Cell> {
        if idx >= 81 {
            None
        } else {
            let row = idx / 9;
            let col = idx % 9;
            Some(&self.cells[row][col])
        }
    }

    /// Get all cells visible to a position (same row, column, or 3x3 box)
    pub fn get_visible_cells(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        let mut cells = Vec::new();

        // Same row
        for c in 0..9 {
            if c != col {
                cells.push((row, c));
            }
        }

        // Same column
        for r in 0..9 {
            if r != row {
                cells.push((r, col));
            }
        }

        // Same 3x3 box
        let box_row = (row / 3) * 3;
        let box_col = (col / 3) * 3;
        for r in box_row..box_row + 3 {
            for c in box_col..box_col + 3 {
                if r != row || c != col {
                    cells.push((r, c));
                }
            }
        }

        cells
    }

    /// Check if two cells can see each other (same row, column, or 3x3 box)
    pub fn can_see(&self, row1: usize, col1: usize, row2: usize, col2: usize) -> bool {
        if row1 == row2 || col1 == col2 {
            return true;
        }
        // Same 3x3 box
        let box_row1 = (row1 / 3) * 3;
        let box_col1 = (col1 / 3) * 3;
        let box_row2 = (row2 / 3) * 3;
        let box_col2 = (col2 / 3) * 3;
        box_row1 == box_row2 && box_col1 == box_col2
    }

    /// Remove a candidate from visible cells
    #[allow(dead_code)]
    pub fn remove_candidate_from_visible(&mut self, row: usize, col: usize, candidate: u8) {
        let visible = Self::get_visible_cells_indices(row, col);
        for (r, c) in visible {
            self.cells[r][c].remove_possible(candidate);
        }
    }

    fn get_visible_cells_indices(row: usize, col: usize) -> Vec<(usize, usize)> {
        let mut cells = Vec::new();

        // Same row
        for c in 0..9 {
            if c != col {
                cells.push((row, c));
            }
        }

        // Same column
        for r in 0..9 {
            if r != row {
                cells.push((r, col));
            }
        }

        // Same 3x3 box
        let box_row = (row / 3) * 3;
        let box_col = (col / 3) * 3;
        for r in box_row..box_row + 3 {
            for c in box_col..box_col + 3 {
                if r != row || c != col {
                    cells.push((r, c));
                }
            }
        }

        cells
    }

    /// Get all unsolved cells
    pub fn get_unsolved(&self) -> Vec<(usize, usize)> {
        let mut unsolved = Vec::new();
        for row in 0..9 {
            for col in 0..9 {
                if !self.cells[row][col].is_solved() {
                    unsolved.push((row, col));
                }
            }
        }
        unsolved
    }

    /// Get all solved cells
    #[allow(dead_code)]
    pub fn get_solved(&self) -> Vec<(usize, usize)> {
        let mut solved = Vec::new();
        for row in 0..9 {
            for col in 0..9 {
                if self.cells[row][col].is_solved() {
                    solved.push((row, col));
                }
            }
        }
        solved
    }

    /// Check if board is fully solved
    pub fn is_solved(&self) -> bool {
        self.unsolved_count == 0
    }

    /// Check if board is in a valid state (no contradictions)
    pub fn is_valid(&self) -> bool {
        // Check no cell has value 0 and is marked impossible
        for row in 0..9 {
            for col in 0..9 {
                let cell = &self.cells[row][col];
                if cell.is_solved() {
                    continue;
                }
                if cell.possible.is_empty() {
                    return false; // Unsolved cell with no candidates = invalid
                }
            }
        }
        true
    }

    /// Get cells in a row
    pub fn get_row(&self, row: usize) -> Option<Vec<&Cell>> {
        if row >= 9 {
            None
        } else {
            Some(self.cells[row].iter().collect())
        }
    }

    /// Get cells in a column
    #[allow(dead_code)]
    pub fn get_column(&self, col: usize) -> Option<Vec<&Cell>> {
        if col >= 9 {
            None
        } else {
            Some((0..9).map(|row| &self.cells[row][col]).collect())
        }
    }

    /// Get cells in a 3x3 box
    pub fn get_box(&self, box_row: usize, box_col: usize) -> Option<Vec<&Cell>> {
        if box_row >= 3 || box_col >= 3 {
            None
        } else {
            let start_row = box_row * 3;
            let start_col = box_col * 3;
            let mut cells = Vec::new();
            for r in start_row..start_row + 3 {
                for c in start_col..start_col + 3 {
                    cells.push(&self.cells[r][c]);
                }
            }
            Some(cells)
        }
    }

    /// Get all cells in board

    /// Count unsolved cells
    pub fn unsolved_count(&self) -> usize {
        self.unsolved_count
    }

    /// Deep clone to create independent copy
    pub fn deep_clone(&self) -> Self {
        Self {
            cells: self.cells.clone(),
            unsolved_count: self.unsolved_count,
        }
    }

    /// Rebuild all candidate values based on current board state
    #[allow(dead_code)]
    pub fn rebuild_candidates(&mut self) {
        // First, clear all candidates
        for row in 0..9 {
            for col in 0..9 {
                if !self.cells[row][col].is_solved() {
                    self.cells[row][col].possible = (1..=9).collect();
                } else {
                    self.cells[row][col].possible.clear();
                }
            }
        }

        // Now remove candidates based on constraints
        for row in 0..9 {
            for col in 0..9 {
                if self.cells[row][col].is_solved() {
                    let value = self.cells[row][col].value;
                    let visible = Self::get_visible_cells_indices(row, col);
                    for (r, c) in visible {
                        self.cells[r][c].remove_possible(value);
                    }
                }
            }
        }
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in 0..9 {
            if row % 3 == 0 && row != 0 {
                writeln!(f, "------+-------+------")?;
            }
            for col in 0..9 {
                if col % 3 == 0 && col != 0 {
                    write!(f, "| ")?;
                }
                let val = self.cells[row][col].value;
                if val == 0 {
                    write!(f, ". ")?;
                } else {
                    write!(f, "{} ", val)?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_creation() {
        let board = Board::new();
        assert_eq!(board.unsolved_count(), 81);
        assert!(!board.is_solved());
    }

    #[test]
    fn test_from_string() {
        let s = "003020600900305001001806400008102900700000008006708200002609500800203006005010300";
        let board = Board::from_string(s).unwrap();
        assert_eq!(board.unsolved_count(), 81 - 27);
        assert!(!board.is_solved());
    }

    #[test]
    fn test_set_cell() {
        let mut board = Board::new();
        board.set_cell(0, 0, 5).unwrap();
        assert_eq!(board.get_cell(0, 0).unwrap().value, 5);
        assert!(!board.get_cell(0, 1).unwrap().possible.contains(&5));
    }

    #[test]
    fn test_visible_cells() {
        let board = Board::new();
        let visible = board.get_visible_cells(4, 4);
        // 8 in row, 8 in col, 4 in box (excluding center)
        assert_eq!(visible.len(), 20);
    }
}
