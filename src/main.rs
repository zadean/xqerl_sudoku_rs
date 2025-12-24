mod board;
mod generator;
mod solver;
mod storage;
mod svg;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        Html, Json,
    },
    routing::{get, post},
    Router,
};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use storage::{PuzzleStore, StoredPuzzle};
use tokio::task;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing_subscriber;

#[derive(Clone)]
pub struct AppState {
    store: PuzzleStore,
}

#[derive(Serialize, Deserialize)]
pub struct CreatePuzzlesRequest {
    count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct PuzzleResponse {
    id: String,
    puzzle_hints: String,
    solved_board: String,
    difficulty: String,
    hint_count: usize,
    winning_play: Vec<solver::WinningMove>,
}

#[derive(Serialize, Deserialize)]
pub struct StatsResponse {
    total_puzzles: usize,
    easy_count: usize,
    medium_count: usize,
    hard_count: usize,
    expert_count: usize,
    avg_hints: usize,
    min_hints: usize,
    max_hints: usize,

    // Algorithm usage counts
    sole_candidate: u32,
    unique_candidate: u32,
    block_column: u32,
    block_row: u32,
    naked_pairs: u32,
    naked_triples: u32,
    hidden_pairs: u32,
    xy_wing: u32,
    x_wing: u32,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let state = AppState {
        store: PuzzleStore::load().await,
    };

    // Serve static files from web directory
    let static_files = ServeDir::new("web");

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/puzzles/create", post(create_puzzles))
        .route("/api/puzzles", get(get_puzzles))
        .route("/api/puzzles/:id", get(get_puzzle))
        .route("/api/puzzles/:id/svg", get(render_puzzle_svg))
        .route(
            "/api/puzzles/difficulty/:difficulty",
            get(get_by_difficulty),
        )
        .route("/api/stats", get(get_stats))
        .route("/api/test", get(test_solver))
        .with_state(state)
        .nest_service("/", static_files)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    tracing::info!("Server listening on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

async fn create_puzzles(
    State(state): State<AppState>,
    Json(req): Json<CreatePuzzlesRequest>,
) -> Sse<impl futures::stream::Stream<Item = Result<Event, std::io::Error>>> {
    let store = state.store.clone();
    let count = req.count;

    let stream = stream::iter(0..count).then(move |_| {
        let store = store.clone();
        async move {
            let puzzle = match task::spawn_blocking(move || {
                loop {
                    // Generate solved board
                    let solved_board = generator::Generator::random_solved_board();

                    // Create puzzle from solved board
                    let (puzzle, solution_id) = generator::Generator::create_puzzle(&solved_board);

                    // Solve puzzle to get difficulty and winning moves
                    let result = solver::Solver::solve(&puzzle);

                    // Validate that the solved board matches
                    if result.board.to_string() != solved_board.to_string() {
                        eprintln!("Puzzle mismatch detected, regenerating...");
                        continue;
                    }

                    let difficulty = result.score.difficulty();
                    let hint_count = generator::Generator::hint_count(&puzzle);
                    let winning_play = result.winning_moves;

                    let stored = StoredPuzzle::new(
                        solution_id,
                        puzzle.to_string(),
                        solved_board.to_string(),
                        result.score,
                        difficulty,
                        hint_count,
                        winning_play,
                    );

                    break stored;
                }
            })
            .await
            {
                Ok(p) => p,
                Err(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Task failed",
                    ))
                }
            };

            // Store puzzle
            store.store(puzzle.clone()).await;

            let response = PuzzleResponse {
                id: puzzle.id,
                puzzle_hints: puzzle.puzzle_hints,
                solved_board: puzzle.solved_board,
                difficulty: puzzle.difficulty,
                hint_count: puzzle.hint_count,
                winning_play: puzzle.winning_play,
            };

            Event::default()
                .json_data(response)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        }
    });

    Sse::new(stream)
}

async fn get_puzzles(State(state): State<AppState>) -> Json<Vec<PuzzleResponse>> {
    let puzzles = state.store.get_all().await;
    let responses = puzzles
        .into_iter()
        .map(|p| PuzzleResponse {
            id: p.id,
            puzzle_hints: p.puzzle_hints,
            solved_board: p.solved_board,
            difficulty: p.difficulty,
            hint_count: p.hint_count,
            winning_play: p.winning_play,
        })
        .collect();
    Json(responses)
}

async fn get_puzzle(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PuzzleResponse>, StatusCode> {
    match state.store.get(&id).await {
        Some(puzzle) => Ok(Json(PuzzleResponse {
            id: puzzle.id,
            puzzle_hints: puzzle.puzzle_hints,
            solved_board: puzzle.solved_board,
            difficulty: puzzle.difficulty,
            hint_count: puzzle.hint_count,
            winning_play: puzzle.winning_play,
        })),
        _ => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_by_difficulty(
    State(state): State<AppState>,
    Path(difficulty): Path<String>,
) -> Json<Vec<PuzzleResponse>> {
    let puzzles = state.store.get_by_difficulty(&difficulty).await;
    let responses = puzzles
        .into_iter()
        .map(|p| PuzzleResponse {
            id: p.id,
            puzzle_hints: p.puzzle_hints,
            solved_board: p.solved_board,
            difficulty: p.difficulty,
            hint_count: p.hint_count,
            winning_play: p.winning_play,
        })
        .collect();
    Json(responses)
}

async fn get_stats(State(state): State<AppState>) -> Json<StatsResponse> {
    let stats = state.store.get_stats().await;
    Json(StatsResponse {
        total_puzzles: stats.total_puzzles,
        easy_count: stats.easy_count,
        medium_count: stats.medium_count,
        hard_count: stats.hard_count,
        expert_count: stats.expert_count,
        avg_hints: stats.avg_hints,
        min_hints: stats.min_hints,
        max_hints: stats.max_hints,
        sole_candidate: stats.sole_candidate,
        unique_candidate: stats.unique_candidate,
        block_column: stats.block_column,
        block_row: stats.block_row,
        naked_pairs: stats.naked_pairs,
        naked_triples: stats.naked_triples,
        hidden_pairs: stats.hidden_pairs,
        xy_wing: stats.xy_wing,
        x_wing: stats.x_wing,
    })
}

async fn test_solver() -> Json<serde_json::Value> {
    // Test with a known difficult puzzle
    let puzzle_str = "4.3.....8.8.3.34.2.2.8.4.91.1.6.9.6.4.1.....6.8.3.7.1....9.8.5.6....7.7.1...";

    match board::Board::from_string(puzzle_str) {
        Ok(board) => {
            let result = task::spawn_blocking(move || solver::Solver::solve(&board))
                .await
                .unwrap();

            Json(serde_json::json!({
                "status": format!("{:?}", result.status),
                "difficulty": result.score.difficulty(),
                "solved": result.status == solver::SolveStatus::Solved,
                "weight": result.score.total_weight(),
            }))
        }
        Err(e) => Json(serde_json::json!({
            "error": e
        })),
    }
}

async fn render_puzzle_svg(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    match state.store.get(&id).await {
        Some(puzzle) => match svg::SvgRenderer::render_puzzle(&puzzle.puzzle_hints) {
            Ok(svg_content) => Ok(Html(svg_content)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
        None => Err(StatusCode::NOT_FOUND),
    }
}
