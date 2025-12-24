use crate::solver::{ScoreMap, WinningMove};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

const PUZZLES_FILE: &str = "puzzles.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredPuzzle {
    pub id: String,
    pub solution_id: String,
    pub puzzle_hints: String,
    pub solved_board: String,
    pub score: ScoreMap,
    pub difficulty: String,
    pub hint_count: usize,
    pub created_at: i64,
    pub winning_play: Vec<WinningMove>,
}

impl StoredPuzzle {
    pub fn new(
        solution_id: String,
        puzzle_hints: String,
        solved_board: String,
        score: ScoreMap,
        difficulty: String,
        hint_count: usize,
        winning_play: Vec<WinningMove>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            solution_id,
            puzzle_hints,
            solved_board,
            score,
            difficulty,
            hint_count,
            created_at: chrono::Local::now().timestamp(),
            winning_play,
        }
    }
}

/// In-memory puzzle store with file persistence
pub struct PuzzleStore {
    puzzles: std::sync::Arc<tokio::sync::RwLock<Vec<StoredPuzzle>>>,
}

impl PuzzleStore {
    pub fn new() -> Self {
        Self {
            puzzles: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Load puzzles from NDJSON file on startup
    pub async fn load() -> Self {
        let store = Self::new();
        if let Ok(file) = fs::File::open(PUZZLES_FILE).await {
            let reader = BufReader::new(file);
            let mut lines = reader.lines();
            let mut puzzles = Vec::new();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(puzzle) = serde_json::from_str::<StoredPuzzle>(&line) {
                    puzzles.push(puzzle);
                }
            }

            if !puzzles.is_empty() {
                let mut puzzles_lock = store.puzzles.write().await;
                *puzzles_lock = puzzles.clone();
                tracing::info!("Loaded {} puzzles from {}", puzzles.len(), PUZZLES_FILE);
            }
        }
        store
    }

    /// Append a puzzle to the NDJSON file
    async fn append_to_file(
        &self,
        puzzle: &StoredPuzzle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(puzzle)?;
        let line = format!("{}\n", json);
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(PUZZLES_FILE)
            .await?
            .write_all(line.as_bytes())
            .await?;
        Ok(())
    }

    /// Store a puzzle and persist to file
    pub async fn store(&self, puzzle: StoredPuzzle) {
        let mut puzzles = self.puzzles.write().await;
        puzzles.push(puzzle.clone());
        drop(puzzles);

        // Append to file asynchronously
        if let Err(e) = self.append_to_file(&puzzle).await {
            tracing::error!("Failed to append puzzle to file: {}", e);
        }
    }

    /// Get all puzzles
    pub async fn get_all(&self) -> Vec<StoredPuzzle> {
        let puzzles = self.puzzles.read().await;
        puzzles.clone()
    }

    /// Get puzzle by ID
    pub async fn get(&self, id: &str) -> Option<StoredPuzzle> {
        let puzzles = self.puzzles.read().await;
        puzzles.iter().find(|p| p.id == id).cloned()
    }

    /// Get puzzles by difficulty
    pub async fn get_by_difficulty(&self, difficulty: &str) -> Vec<StoredPuzzle> {
        let puzzles = self.puzzles.read().await;
        puzzles
            .iter()
            .filter(|p| p.difficulty == difficulty)
            .cloned()
            .collect()
    }

    /// Get the count of stored puzzles
    #[allow(dead_code)]
    pub async fn count(&self) -> usize {
        let puzzles = self.puzzles.read().await;
        puzzles.len()
    }

    /// Get aggregated statistics
    pub async fn get_stats(&self) -> StoreStats {
        let puzzles = self.puzzles.read().await;

        let mut stats = StoreStats::default();
        let mut min_hints = usize::MAX;
        let mut max_hints = 0;

        for puzzle in puzzles.iter() {
            stats.total_puzzles += 1;

            match puzzle.difficulty.as_str() {
                "Easy" => stats.easy_count += 1,
                "Medium" => stats.medium_count += 1,
                "Hard" => stats.hard_count += 1,
                "Expert" => stats.expert_count += 1,
                _ => {}
            }

            stats.total_hints += puzzle.hint_count;
            min_hints = min_hints.min(puzzle.hint_count);
            max_hints = max_hints.max(puzzle.hint_count);

            // Accumulate scores
            stats.sole_candidate += puzzle.score.sole_candidate;
            stats.unique_candidate += puzzle.score.unique_candidate;
            stats.block_column += puzzle.score.block_column;
            stats.block_row += puzzle.score.block_row;
            stats.naked_pairs += puzzle.score.naked_pairs;
            stats.naked_triples += puzzle.score.naked_triples;
            stats.hidden_pairs += puzzle.score.hidden_pairs;
            stats.xy_wing += puzzle.score.xy_wing;
            stats.x_wing += puzzle.score.x_wing;
        }

        if stats.total_puzzles > 0 {
            stats.avg_hints = stats.total_hints / stats.total_puzzles;
            stats.min_hints = min_hints;
            stats.max_hints = max_hints;
        }

        stats
    }
}

impl Clone for PuzzleStore {
    fn clone(&self) -> Self {
        Self {
            puzzles: self.puzzles.clone(),
        }
    }
}

impl Default for PuzzleStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StoreStats {
    pub total_puzzles: usize,
    pub easy_count: usize,
    pub medium_count: usize,
    pub hard_count: usize,
    pub expert_count: usize,
    pub total_hints: usize,
    pub avg_hints: usize,
    pub min_hints: usize,
    pub max_hints: usize,

    // Technique counts
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let store = PuzzleStore::new();
        let puzzle = StoredPuzzle::new(
            "sol123".to_string(),
            "003020600900305001001806400008102900700000008006708200002609500800203006005010300"
                .to_string(),
            "123456789456789123789123456234567891567891234891234567345678912678912345912345678"
                .to_string(),
            ScoreMap::default(),
            "Easy".to_string(),
            27,
        );

        store.store(puzzle.clone()).await;
        assert_eq!(store.count().await, 1);

        let retrieved = store.get(&puzzle.id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, puzzle.id);
    }

    #[tokio::test]
    async fn test_get_by_difficulty() {
        let store = PuzzleStore::new();
        let easy = StoredPuzzle::new(
            "sol1".to_string(),
            "test".to_string(),
            "test".to_string(),
            ScoreMap::default(),
            "Easy".to_string(),
            27,
        );
        let hard = StoredPuzzle::new(
            "sol2".to_string(),
            "test".to_string(),
            "test".to_string(),
            ScoreMap::default(),
            "Hard".to_string(),
            17,
        );

        store.store(easy).await;
        store.store(hard).await;

        assert_eq!(store.get_by_difficulty("Easy").await.len(), 1);
        assert_eq!(store.get_by_difficulty("Hard").await.len(), 1);
    }

    #[tokio::test]
    async fn test_stats() {
        let store = PuzzleStore::new();
        let puzzle = StoredPuzzle::new(
            "sol1".to_string(),
            "test".to_string(),
            "test".to_string(),
            ScoreMap::default(),
            "Easy".to_string(),
            27,
        );

        store.store(puzzle).await;
        let stats = store.get_stats().await;

        assert_eq!(stats.total_puzzles, 1);
        assert_eq!(stats.easy_count, 1);
        assert_eq!(stats.avg_hints, 27);
    }
}
