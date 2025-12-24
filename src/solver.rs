use crate::board::Board;
use std::collections::{HashMap, HashSet};

/// Represents a single step to win the puzzle
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum WinningMove {
    #[serde(rename = "placement")]
    CellPlaced {
        cell_index: usize, // 0-80 representing position in 81-cell board
        value: u8,         // The number placed (1-9)
        technique: String, // The solving technique used
    },
    #[serde(rename = "removal")]
    CandidateRemoved {
        cell_index: usize,   // 0-80 representing position in 81-cell board
        candidates: Vec<u8>, // The candidates removed (1-9)
        reason: String,      // Why they were removed (e.g., "Unique Candidate at 23")
    },
}

/// Tracks which techniques were used and their frequency
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ScoreMap {
    pub sole_candidate: u32,
    pub unique_candidate: u32,
    pub block_column: u32,
    pub block_row: u32,
    pub naked_pairs: u32,
    pub naked_triples: u32,
    pub hidden_pairs: u32,
    pub xy_wing: u32,
    pub x_wing: u32,
}

impl ScoreMap {
    pub fn total_weight(&self) -> u32 {
        self.sole_candidate * 1
            + self.unique_candidate * 1
            + self.block_column * 1
            + self.block_row * 1
            + self.naked_pairs * 10
            + self.naked_triples * 10
            + self.hidden_pairs * 10
            + self.xy_wing * 100
            + self.x_wing * 100
    }

    pub fn difficulty(&self) -> String {
        // Expert: if uses xy_wing, coloring, or forcing_chain
        if self.xy_wing > 0 {
            return "Expert".to_string();
        }

        // Hard: if uses hidden_4, hidden_3, naked_4, chain_1, xy_wing, x_wing
        if self.hidden_pairs > 0 || self.naked_triples > 0 || self.x_wing > 0 {
            return "Hard".to_string();
        }

        // Medium: if uses hidden_2, naked_3, block, or naked_2
        if self.hidden_pairs > 0
            || self.naked_pairs > 0
            || self.block_column > 0
            || self.block_row > 0
        {
            return "Medium".to_string();
        }

        // Easy: otherwise
        "Easy".to_string()
    }
}

/// Result of a solve attempt
#[derive(Clone, Debug)]
pub struct SolveResult {
    #[allow(dead_code)]
    pub board: Board,
    pub score: ScoreMap,
    pub status: SolveStatus,
    pub winning_moves: Vec<WinningMove>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SolveStatus {
    Solved,
    Unsolved,
    Invalid,
}

pub struct Solver;

impl Solver {
    /// Main solve function - applies all techniques iteratively
    pub fn solve(board: &Board) -> SolveResult {
        let mut board = board.deep_clone();
        let mut score = ScoreMap::default();
        let mut winning_moves = Vec::new();
        let mut changed = true;

        while changed && board.unsolved_count() > 0 && board.is_valid() {
            changed = false;

            // Snapshot board state before each technique attempt
            let board_before = board.deep_clone();

            // First, apply constraint propagation techniques (eliminate candidates)
            if Self::apply_unique_candidate(
                &mut board,
                &mut score,
                &mut winning_moves,
                &board_before,
            ) {
                changed = true;
                continue;
            }

            if Self::apply_block_interactions(
                &mut board,
                &mut score,
                &mut winning_moves,
                &board_before,
            ) {
                changed = true;
                continue;
            }

            // Try medium constraint techniques
            if Self::apply_naked_subsets(&mut board, &mut score, &mut winning_moves, &board_before)
            {
                changed = true;
                continue;
            }

            if Self::apply_hidden_subsets(&mut board, &mut score, &mut winning_moves, &board_before)
            {
                changed = true;
                continue;
            }

            // Try advanced constraint techniques
            if Self::apply_wing_patterns(&mut board, &mut score, &mut winning_moves, &board_before)
            {
                changed = true;
                continue;
            }

            if Self::apply_x_patterns(&mut board, &mut score, &mut winning_moves, &board_before) {
                changed = true;
                continue;
            }

            // Only after all constraint propagation, check for cells with single candidate
            if Self::apply_sole_candidate(&mut board, &mut score, &mut winning_moves, &board_before)
            {
                changed = true;
                continue;
            }
        }

        let status = if !board.is_valid() {
            SolveStatus::Invalid
        } else if board.is_solved() {
            SolveStatus::Solved
        } else {
            SolveStatus::Unsolved
        };

        SolveResult {
            board,
            score,
            status,
            winning_moves,
        }
    }

    /// Sole Candidate: If a cell has only one possible value, set it
    fn apply_sole_candidate(
        board: &mut Board,
        score: &mut ScoreMap,
        winning_moves: &mut Vec<WinningMove>,
        board_before: &Board,
    ) -> bool {
        for (row, col) in board.get_unsolved() {
            if let Some(cell) = board.get_cell(row, col) {
                if cell.num_candidates() == 1 {
                    let value = *cell.possible.iter().next().unwrap();
                    let cell_index = row * 9 + col;
                    board.set_cell(row, col, value).ok();
                    score.sole_candidate += 1;
                    winning_moves.push(WinningMove::CellPlaced {
                        cell_index,
                        value,
                        technique: "Sole Candidate".to_string(),
                    });

                    // Record actual candidate removals by comparing board state
                    Self::record_candidate_removals(board_before, board, cell_index, winning_moves);

                    return true;
                }
            }
        }
        false
    }

    /// Helper to record actual candidate removals between two board states
    fn record_candidate_removals(
        board_before: &Board,
        board_after: &Board,
        placement_index: usize,
        winning_moves: &mut Vec<WinningMove>,
    ) {
        for idx in 0..81 {
            if let (Some(cell_before), Some(cell_after)) = (
                board_before.get_cell_by_idx(idx),
                board_after.get_cell_by_idx(idx),
            ) {
                if idx != placement_index && !cell_after.is_solved() {
                    let removed: Vec<u8> = cell_before
                        .possible
                        .iter()
                        .filter(|c| !cell_after.possible.contains(c))
                        .copied()
                        .collect();

                    if !removed.is_empty() {
                        winning_moves.push(WinningMove::CandidateRemoved {
                            cell_index: idx,
                            candidates: removed,
                            reason: format!("Placement at cell {}", placement_index),
                        });
                    }
                }
            }
        }
    }

    /// Helper to record candidate removals from constraint techniques (no placement)
    fn record_constraint_removals(
        board_before: &Board,
        board_after: &Board,
        technique: &str,
        winning_moves: &mut Vec<WinningMove>,
    ) {
        for idx in 0..81 {
            if let (Some(cell_before), Some(cell_after)) = (
                board_before.get_cell_by_idx(idx),
                board_after.get_cell_by_idx(idx),
            ) {
                if !cell_after.is_solved() {
                    let removed: Vec<u8> = cell_before
                        .possible
                        .iter()
                        .filter(|c| !cell_after.possible.contains(c))
                        .copied()
                        .collect();

                    if !removed.is_empty() {
                        winning_moves.push(WinningMove::CandidateRemoved {
                            cell_index: idx,
                            candidates: removed,
                            reason: technique.to_string(),
                        });
                    }
                }
            }
        }
    }

    /// Helper to record constraint removals with specific cell involvement details
    fn record_constraint_removals_with_cells(
        board_before: &Board,
        board_after: &Board,
        technique: &str,
        cells_involved: &[usize],
        winning_moves: &mut Vec<WinningMove>,
    ) {
        for idx in 0..81 {
            if let (Some(cell_before), Some(cell_after)) = (
                board_before.get_cell_by_idx(idx),
                board_after.get_cell_by_idx(idx),
            ) {
                if !cell_after.is_solved() {
                    let removed: Vec<u8> = cell_before
                        .possible
                        .iter()
                        .filter(|c| !cell_after.possible.contains(c))
                        .copied()
                        .collect();

                    if !removed.is_empty() {
                        let cell_str = cells_involved
                            .iter()
                            .map(|c| c.to_string())
                            .collect::<Vec<_>>()
                            .join(",");
                        let reason = format!("{} using cells {}", technique, cell_str);
                        winning_moves.push(WinningMove::CandidateRemoved {
                            cell_index: idx,
                            candidates: removed,
                            reason,
                        });
                    }
                }
            }
        }
    }

    /// Unique Candidate: If a value appears in only one cell in a unit, set it
    fn apply_unique_candidate(
        board: &mut Board,
        score: &mut ScoreMap,
        winning_moves: &mut Vec<WinningMove>,
        board_before: &Board,
    ) -> bool {
        // Check rows
        for row in 0..9 {
            for digit in 1..=9 {
                let positions: Vec<usize> = (0..9)
                    .filter(|col| {
                        if let Some(cell) = board.get_cell(row, *col) {
                            cell.possible.contains(&digit)
                        } else {
                            false
                        }
                    })
                    .collect();

                if positions.len() == 1 {
                    let col = positions[0];
                    if let Some(cell) = board.get_cell(row, col) {
                        if !cell.is_solved() {
                            let cell_index = row * 9 + col;
                            board.set_cell(row, col, digit).ok();
                            score.unique_candidate += 1;
                            winning_moves.push(WinningMove::CellPlaced {
                                cell_index,
                                value: digit,
                                technique: "Unique Candidate".to_string(),
                            });

                            Self::record_candidate_removals(
                                board_before,
                                board,
                                cell_index,
                                winning_moves,
                            );
                            return true;
                        }
                    }
                }
            }
        }

        // Check columns
        for col in 0..9 {
            for digit in 1..=9 {
                let positions: Vec<usize> = (0..9)
                    .filter(|row| {
                        if let Some(cell) = board.get_cell(*row, col) {
                            cell.possible.contains(&digit)
                        } else {
                            false
                        }
                    })
                    .collect();

                if positions.len() == 1 {
                    let row = positions[0];
                    if let Some(cell) = board.get_cell(row, col) {
                        if !cell.is_solved() {
                            let cell_index = row * 9 + col;
                            board.set_cell(row, col, digit).ok();
                            score.unique_candidate += 1;
                            winning_moves.push(WinningMove::CellPlaced {
                                cell_index,
                                value: digit,
                                technique: "Unique Candidate".to_string(),
                            });

                            Self::record_candidate_removals(
                                board_before,
                                board,
                                cell_index,
                                winning_moves,
                            );
                            return true;
                        }
                    }
                }
            }
        }

        // Check boxes
        for box_row in 0..3 {
            for box_col in 0..3 {
                for digit in 1..=9 {
                    let positions: Vec<(usize, usize)> =
                        if let Some(cells) = board.get_box(box_row, box_col) {
                            cells
                                .iter()
                                .enumerate()
                                .filter(|(_, cell)| cell.possible.contains(&digit))
                                .map(|(idx, _)| {
                                    let r = box_row * 3 + idx / 3;
                                    let c = box_col * 3 + idx % 3;
                                    (r, c)
                                })
                                .collect()
                        } else {
                            Vec::new()
                        };

                    if positions.len() == 1 {
                        let (row, col) = positions[0];
                        if let Some(cell) = board.get_cell(row, col) {
                            if !cell.is_solved() {
                                let cell_index = row * 9 + col;
                                board.set_cell(row, col, digit).ok();
                                score.unique_candidate += 1;
                                winning_moves.push(WinningMove::CellPlaced {
                                    cell_index,
                                    value: digit,
                                    technique: "Unique Candidate".to_string(),
                                });

                                Self::record_candidate_removals(
                                    board_before,
                                    board,
                                    cell_index,
                                    winning_moves,
                                );
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Block interactions: If a digit in a box can only be in one row/column
    fn apply_block_interactions(
        board: &mut Board,
        score: &mut ScoreMap,
        winning_moves: &mut Vec<WinningMove>,
        board_before: &Board,
    ) -> bool {
        let mut found_any = false;

        for box_row in 0..3 {
            for box_col in 0..3 {
                for digit in 1..=9 {
                    // Find rows where this digit can appear in this box
                    let mut row_candidates = HashSet::new();
                    if let Some(cells) = board.get_box(box_row, box_col) {
                        for (idx, cell) in cells.iter().enumerate() {
                            if cell.possible.contains(&digit) {
                                let r = box_row * 3 + idx / 3;
                                row_candidates.insert(r);
                            }
                        }
                    }

                    if row_candidates.len() == 1 {
                        let row = *row_candidates.iter().next().unwrap();
                        let start_col = box_col * 3;
                        for col in 0..9 {
                            if col < start_col || col >= start_col + 3 {
                                if let Some(cell) = board.get_cell_mut(row, col) {
                                    if cell.possible.remove(&digit) {
                                        score.block_row += 1;
                                        found_any = true;
                                    }
                                }
                            }
                        }
                    }

                    // Find columns where this digit can appear
                    let mut col_candidates = HashSet::new();
                    if let Some(cells) = board.get_box(box_row, box_col) {
                        for (idx, cell) in cells.iter().enumerate() {
                            if cell.possible.contains(&digit) {
                                let c = box_col * 3 + idx % 3;
                                col_candidates.insert(c);
                            }
                        }
                    }

                    if col_candidates.len() == 1 {
                        let col = *col_candidates.iter().next().unwrap();
                        let start_row = box_row * 3;
                        for row in 0..9 {
                            if row < start_row || row >= start_row + 3 {
                                if let Some(cell) = board.get_cell_mut(row, col) {
                                    if cell.possible.remove(&digit) {
                                        score.block_column += 1;
                                        found_any = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Track removals
        if found_any {
            Self::record_constraint_removals(
                board_before,
                board,
                "Block Interaction",
                winning_moves,
            );
        }

        found_any
    }

    /// Naked pairs/triples: If N cells have the same N candidates, remove those from peers
    /// Naked pairs/triples: If N cells have the same N candidates, remove those from peers
    fn apply_naked_subsets(
        board: &mut Board,
        score: &mut ScoreMap,
        winning_moves: &mut Vec<WinningMove>,
        board_before: &Board,
    ) -> bool {
        let board_before_state = board_before.deep_clone();

        // Check naked pairs in rows
        for row in 0..9 {
            if let Some(cells) = board.get_row(row) {
                let candidates: Vec<(usize, HashSet<u8>)> = cells
                    .iter()
                    .enumerate()
                    .filter(|(_, cell)| {
                        !cell.is_solved()
                            && cell.num_candidates() >= 2
                            && cell.num_candidates() <= 3
                    })
                    .map(|(col, cell)| (col, cell.possible.clone()))
                    .collect();

                // Check for pairs
                for i in 0..candidates.len() {
                    for j in (i + 1)..candidates.len() {
                        if candidates[i].1 == candidates[j].1 && candidates[i].1.len() == 2 {
                            let pair = candidates[i].1.clone();
                            let cell1_idx = row * 9 + candidates[i].0;
                            let cell2_idx = row * 9 + candidates[j].0;
                            let mut removed = false;
                            for col in 0..9 {
                                if col != candidates[i].0 && col != candidates[j].0 {
                                    if let Some(cell) = board.get_cell_mut(row, col) {
                                        for &digit in &pair {
                                            if cell.possible.remove(&digit) {
                                                removed = true;
                                            }
                                        }
                                    }
                                }
                            }
                            if removed {
                                score.naked_pairs += 1;
                                Self::record_constraint_removals_with_cells(
                                    &board_before_state,
                                    board,
                                    "Naked Pair",
                                    &[cell1_idx, cell2_idx],
                                    winning_moves,
                                );
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Hidden subsets: If N values can only appear in N cells, remove other candidates
    fn apply_hidden_subsets(
        board: &mut Board,
        score: &mut ScoreMap,
        winning_moves: &mut Vec<WinningMove>,
        board_before: &Board,
    ) -> bool {
        let board_before_state = board_before.deep_clone();

        // Check hidden pairs in rows
        for row in 0..9 {
            let mut digit_positions: HashMap<u8, Vec<usize>> = HashMap::new();
            for col in 0..9 {
                if let Some(cell) = board.get_cell(row, col) {
                    if !cell.is_solved() {
                        for &digit in &cell.possible {
                            digit_positions
                                .entry(digit)
                                .or_insert_with(Vec::new)
                                .push(col);
                        }
                    }
                }
            }

            for digit1 in 1..=9 {
                if let Some(pos1) = digit_positions.get(&digit1) {
                    if pos1.len() == 2 {
                        for digit2 in (digit1 + 1)..=9 {
                            if let Some(pos2) = digit_positions.get(&digit2) {
                                if pos2.len() == 2 && pos1 == pos2 {
                                    let pair = vec![digit1, digit2];
                                    let cells_involved: Vec<usize> =
                                        pos1.iter().map(|col| row * 9 + col).collect();
                                    let mut removed = false;
                                    for &col in pos1 {
                                        if let Some(cell) = board.get_cell_mut(row, col) {
                                            for digit in 1..=9 {
                                                if !pair.contains(&digit)
                                                    && cell.possible.remove(&digit)
                                                {
                                                    removed = true;
                                                }
                                            }
                                        }
                                    }
                                    if removed {
                                        score.hidden_pairs += 1;
                                        Self::record_constraint_removals_with_cells(
                                            &board_before_state,
                                            board,
                                            "Hidden Pair",
                                            &cells_involved,
                                            winning_moves,
                                        );
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Y-Wing: Find a pivot with 2 candidates and two pincers that see each other but don't see pivot
    fn apply_wing_patterns(
        board: &mut Board,
        score: &mut ScoreMap,
        winning_moves: &mut Vec<WinningMove>,
        board_before: &Board,
    ) -> bool {
        let board_before_state = board_before.deep_clone();

        // Y-Wing: Find a pivot cell (AB) with exactly 2 candidates
        let unsolved_cells: Vec<(usize, usize)> = board.get_unsolved();

        for (pivot_row, pivot_col) in unsolved_cells {
            if let Some(pivot_cell) = board.get_cell(pivot_row, pivot_col) {
                if pivot_cell.num_candidates() != 2 {
                    continue;
                }

                let pivot_index = pivot_row * 9 + pivot_col;
                let pivot_candidates = pivot_cell.possible.clone();
                let mut pivot_vec: Vec<u8> = pivot_candidates.iter().copied().collect();
                pivot_vec.sort();
                let (a, b) = (pivot_vec[0], pivot_vec[1]);

                // Find first pincer with (A, C) candidates that sees pivot
                let visible_to_pivot = board.get_visible_cells(pivot_row, pivot_col);
                for (pincer1_row, pincer1_col) in visible_to_pivot.clone() {
                    if let Some(pincer1_cell) = board.get_cell(pincer1_row, pincer1_col) {
                        if pincer1_cell.num_candidates() != 2 {
                            continue;
                        }

                        let pincer1_candidates = pincer1_cell.possible.clone();

                        // Pincer1 must contain one of A or B and a different candidate C
                        let contains_a = pincer1_candidates.contains(&a);
                        let contains_b = pincer1_candidates.contains(&b);

                        if !(contains_a ^ contains_b) {
                            // Must contain exactly one of A or B
                            continue;
                        }

                        let common_digit = if contains_a { a } else { b };
                        let mut pincer1_vec: Vec<u8> = pincer1_candidates.iter().copied().collect();
                        pincer1_vec.sort();
                        let c = pincer1_vec
                            .iter()
                            .find(|&&d| d != common_digit)
                            .copied()
                            .unwrap();

                        let pincer1_index = pincer1_row * 9 + pincer1_col;

                        // Find second pincer with (B, C) or (A, C) that sees pivot but NOT pincer1
                        for (pincer2_row, pincer2_col) in visible_to_pivot.clone() {
                            if pincer2_row == pincer1_row && pincer2_col == pincer1_col {
                                continue;
                            }

                            if let Some(pincer2_cell) = board.get_cell(pincer2_row, pincer2_col) {
                                if pincer2_cell.num_candidates() != 2 {
                                    continue;
                                }

                                let pincer2_candidates = pincer2_cell.possible.clone();

                                // Pincer2 must contain the OTHER digit from pivot (A or B) and C
                                let other_digit = if contains_a { b } else { a };
                                if !pincer2_candidates.contains(&other_digit)
                                    || !pincer2_candidates.contains(&c)
                                {
                                    continue;
                                }

                                // Pincers must NOT see each other
                                if board.can_see(pincer1_row, pincer1_col, pincer2_row, pincer2_col)
                                {
                                    continue;
                                }

                                let pincer2_index = pincer2_row * 9 + pincer2_col;

                                // Found a Y-Wing! Remove C from cells that see both pincers
                                let mut removed = false;
                                for row in 0..9 {
                                    for col in 0..9 {
                                        if (row == pincer1_row && col == pincer1_col)
                                            || (row == pincer2_row && col == pincer2_col)
                                            || (row == pivot_row && col == pivot_col)
                                        {
                                            continue;
                                        }

                                        // Check visibility before borrowing
                                        let sees_both =
                                            board.can_see(row, col, pincer1_row, pincer1_col)
                                                && board.can_see(
                                                    row,
                                                    col,
                                                    pincer2_row,
                                                    pincer2_col,
                                                );

                                        if sees_both {
                                            if let Some(cell) = board.get_cell_mut(row, col) {
                                                if cell.possible.remove(&c) {
                                                    removed = true;
                                                }
                                            }
                                        }
                                    }
                                }

                                if removed {
                                    score.xy_wing += 1;
                                    Self::record_constraint_removals_with_cells(
                                        &board_before_state,
                                        board,
                                        "Y-Wing",
                                        &[pivot_index, pincer1_index, pincer2_index],
                                        winning_moves,
                                    );
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// X-patterns: X-Wing, X-Cycle
    fn apply_x_patterns(
        board: &mut Board,
        score: &mut ScoreMap,
        winning_moves: &mut Vec<WinningMove>,
        board_before: &Board,
    ) -> bool {
        let board_before_state = board_before.deep_clone();

        // X-Wing: if a digit appears in exactly 2 cells in each of 2 rows, and they share columns
        // Check rows -> eliminate in columns
        for digit in 1..=9 {
            let mut row_positions: HashMap<usize, Vec<usize>> = HashMap::new();

            for row in 0..9 {
                let cols: Vec<usize> = (0..9)
                    .filter(|col| {
                        if let Some(cell) = board.get_cell(row, *col) {
                            cell.possible.contains(&digit)
                        } else {
                            false
                        }
                    })
                    .collect();

                if cols.len() == 2 {
                    row_positions.insert(row, cols);
                }
            }

            // Check if two rows share the same columns
            let rows: Vec<usize> = row_positions.keys().copied().collect();
            for i in 0..rows.len() {
                for j in (i + 1)..rows.len() {
                    let cols1 = &row_positions[&rows[i]];
                    let cols2 = &row_positions[&rows[j]];

                    if cols1 == cols2 {
                        // Found X-Wing, remove digit from other cells in these columns
                        let mut cells_involved = vec![
                            rows[i] * 9 + cols1[0],
                            rows[i] * 9 + cols1[1],
                            rows[j] * 9 + cols1[0],
                            rows[j] * 9 + cols1[1],
                        ];
                        cells_involved.sort();
                        let mut removed = false;
                        for &col in cols1 {
                            for row in 0..9 {
                                if row != rows[i] && row != rows[j] {
                                    if let Some(cell) = board.get_cell_mut(row, col) {
                                        if cell.possible.remove(&digit) {
                                            removed = true;
                                        }
                                    }
                                }
                            }
                        }
                        if removed {
                            score.x_wing += 1;
                            Self::record_constraint_removals_with_cells(
                                &board_before_state,
                                board,
                                "X-Wing",
                                &cells_involved,
                                winning_moves,
                            );
                            return true;
                        }
                    }
                }
            }
        }

        // X-Wing: if a digit appears in exactly 2 cells in each of 2 columns, and they share rows
        // Check columns -> eliminate in rows
        for digit in 1..=9 {
            let mut col_positions: HashMap<usize, Vec<usize>> = HashMap::new();

            for col in 0..9 {
                let rows: Vec<usize> = (0..9)
                    .filter(|row| {
                        if let Some(cell) = board.get_cell(*row, col) {
                            cell.possible.contains(&digit)
                        } else {
                            false
                        }
                    })
                    .collect();

                if rows.len() == 2 {
                    col_positions.insert(col, rows);
                }
            }

            // Check if two columns share the same rows
            let cols: Vec<usize> = col_positions.keys().copied().collect();
            for i in 0..cols.len() {
                for j in (i + 1)..cols.len() {
                    let rows1 = &col_positions[&cols[i]];
                    let rows2 = &col_positions[&cols[j]];

                    if rows1 == rows2 {
                        // Found X-Wing, remove digit from other cells in these rows
                        let mut cells_involved = vec![
                            rows1[0] * 9 + cols[i],
                            rows1[0] * 9 + cols[j],
                            rows1[1] * 9 + cols[i],
                            rows1[1] * 9 + cols[j],
                        ];
                        cells_involved.sort();
                        let mut removed = false;
                        for &row in rows1 {
                            for col in 0..9 {
                                if col != cols[i] && col != cols[j] {
                                    if let Some(cell) = board.get_cell_mut(row, col) {
                                        if cell.possible.remove(&digit) {
                                            removed = true;
                                        }
                                    }
                                }
                            }
                        }
                        if removed {
                            score.x_wing += 1;
                            Self::record_constraint_removals_with_cells(
                                &board_before_state,
                                board,
                                "X-Wing",
                                &cells_involved,
                                winning_moves,
                            );
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Solve with medium techniques (simple + subsets)
    #[allow(dead_code)]
    pub fn solve_medium(board: &Board) -> SolveResult {
        let mut board = board.deep_clone();
        let mut score = ScoreMap::default();
        let mut winning_moves = Vec::new();
        let mut changed = true;

        while changed && board.unsolved_count() > 0 && board.is_valid() {
            changed = false;
            let board_before = board.deep_clone();

            if Self::apply_sole_candidate(&mut board, &mut score, &mut winning_moves, &board_before)
            {
                changed = true;
            } else if Self::apply_unique_candidate(
                &mut board,
                &mut score,
                &mut winning_moves,
                &board_before,
            ) {
                changed = true;
            } else if Self::apply_block_interactions(
                &mut board,
                &mut score,
                &mut winning_moves,
                &board_before,
            ) {
                changed = true;
            } else if Self::apply_naked_subsets(
                &mut board,
                &mut score,
                &mut winning_moves,
                &board_before,
            ) {
                changed = true;
            } else if Self::apply_hidden_subsets(
                &mut board,
                &mut score,
                &mut winning_moves,
                &board_before,
            ) {
                changed = true;
            }
        }

        let status = if !board.is_valid() {
            SolveStatus::Invalid
        } else if board.is_solved() {
            SolveStatus::Solved
        } else {
            SolveStatus::Unsolved
        };

        SolveResult {
            board,
            score,
            status,
            winning_moves,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sole_candidate() {
        let s = "003020600900305001001806400008102900700000008006708200002609500800203006005010300";
        let board = Board::from_string(s).unwrap();
        let result = Solver::solve(&board);
        assert_eq!(result.status, SolveStatus::Solved);
    }

    #[test]
    fn test_solve_simple() {
        let s = "003020600900305001001806400008102900700000008006708200002609500800203006005010300";
        let board = Board::from_string(s).unwrap();
        let result = Solver::solve_medium(&board);
        // May not be fully solved with simple techniques
        assert!(result.board.is_valid());
    }
}
