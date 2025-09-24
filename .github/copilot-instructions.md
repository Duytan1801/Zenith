# Copilot Instructions for Trying Chess Project

## Project Overview

This Rust project simulates chess games starting from random openings loaded from `openings_sheet.csv`, then plays out the middlegame and endgame using a simple AI engine.

### Major Components
- **`src/main.rs`**: Entry point. Initializes `pleco::Board`, parses CSV for openings (ECO, name, SAN moves) using a custom quote-aware parser, selects one randomly with `rand`, applies moves via a wrapper `parse_san()` around pleco's parser, then enters game loop calling `engine::get_best_move()` until checkmate/stalemate.
- **`src/engine.rs`**: AI implementation. Key functions:
  - `evaluate_board()`: Material (P=100, N=320, B=330, R=500, Q=900, K=20000) + midgame PSQT tables (constants like `PSQT_PAWN_MG`). Returns score from the perspective of the side to move (positive advantage).
  - `minimax()`: Recursive alpha-beta search with null-move pruning (depth >=3, for non-maximizing player), transposition table (`HashMap<u64, TTEntry>` for exact/lower/upper bounds), and quiescence at depth 0 using SEE >=0 for captures.
  - `get_best_move()`: Iterative deepening (depths 1 to `MAX_DEPTH=5`), static move ordering (MVV-LVA captures, killers, history), returns best `BitMove` after full search.
- **`openings_sheet.csv`**: Data source. Format: `ECO,name,\"move1 move2 ...\"` (e.g., `A00,English Opening,\"1. c4 e5\"`). Skips header, filters empty lines.

### Architecture and Data Flow
- Start: `Board::start_pos()` → Parse CSV with byte-level quote handling → Random opening selection → Apply SAN moves in loop using `parse_san` wrapper and `board.apply_move()`.
- Game loop: While not terminal, AI computes `get_best_move(&mut board, MAX_DEPTH)` (modifies board temporarily via apply/undo) → Apply best move → Print board state.
- Why this structure? Separates opening book (CSV for variety/diversity) from dynamic search (engine.rs for computation). Single-threaded for simplicity; no UCI or parallelism.
- Boundaries: `main.rs` handles I/O, CSV parsing, and game loop; `engine.rs` is pure computation (relies on `pleco` for board state, uses unsafe globals for heuristics).

## Critical Workflows
- **Build and Run**: `cargo build` (standard Rust). `cargo run` loads CSV, picks random opening, simulates full game to terminal (checkmate/stalemate/draw), prints moves and board states via `{:?}`.
- **Debug AI**: Adjust `MAX_DEPTH` in `engine.rs` (u8, default 5) to tune search depth. Add `println!` in `minimax` or `quiescence` for node counts/PV tracing. Use `board.pretty_print()` for visual board output if available, or `{:?}` for debug.
- **Test Openings**: Edit `openings_sheet.csv` manually, run `cargo run`. The `parse_san` wrapper propagates pleco's handling of castling (O-O), promotions, checks (+/#); invalid SAN logs error and skips remaining moves (no panic).
- **Profile Search**: No built-in profiling; install `cargo-flamegraph` (`cargo install flamegraph`) and run `cargo flamegraph` to identify hotspots in `minimax` or evaluation.
- No unit tests present; extend with `cargo test` for functions like `evaluate_board` on benchmark positions if needed.

## Project-Specific Conventions
- **Move Handling**: SAN strings from CSV (e.g., `"e4"`, `"Nf3"`, `"O-O"`); parse with `parse_san` wrapper (adds error formatting to pleco's parser, which handles cleaning +/#, piece disambiguation, castling). AI outputs `BitMove`; apply via `board.apply_move()`, undo via `board.undo_move()`. Use `board.generate_pseudolegal_moves()` filtered by `legal_move()` for search.
- **Scoring**: Side-to-move positive (white pieces summed positive, black negative, negated if black to move). `INFINITY = 1_000_000`. TT flags: 0=exact, 1=lower, 2=upper.
- **Heuristics**: Unsafe static globals for killers (`KILLERS[MAX_PLY][2]`) and history (`HISTORY[6][64]` for piece-type to destination square). Updated on beta cutoffs per ply (`board.ply() % MAX_PLY`); supports multi-ply search.
- **Ordering**: `score_move()`: Captures via MVV-LVA (10_000 + victim_value*100 - attacker_value), killers (9_000), history +1_000. `order_moves` sorts descending before each search iteration.
- **Pruning/Search**: Standard alpha-beta with early cutoff and bound updates; null-move (depth reduction by 3) for non-maximizing player if not in check. Quiescence extends captures (filtered from all pseudolegal moves with SEE >=0) to avoid horizon effect.
- Differs from standard engines: Simple `HashMap` TT (no age/replacement policy, grows unbounded); midgame-only PSQT (no endgame tables); iterative deepening without PV-based reordering; globals for performance over thread-safety. Root move ordering is static (recomputed each depth but identical).
- Example pattern in `engine.rs` (move ordering):
  ```rust
  pub fn order_moves(moves: &mut Vec<BitMove>, board: &Board) {
      moves.sort_by_key(|&m| std::cmp::Reverse(score_move(m, board)));
  }
  ```
- CSV Parsing: Custom byte-loop parser in `parse_csv_line` handles quoted fields (e.g., names with commas); splits moves via `split_whitespace()`, then filters out move numbers (e.g., '1.') to extract pure SAN like 'c4'. No external CSV crate for lightweight dependency.

## Integrations and Communication
- **External Deps**:
  - `pleco = "0.5.0"`: Chess engine backend (board representation, move generation, Zobrist hashing, SEE). All board operations via `pleco`; avoid reimplementing (e.g., use `board.zobrist()` for TT keys).
  - `rand = "0.9.2"`: Random opening selection via `Uniform` distribution and `thread_rng()`.
- **Cross-Component**: `main.rs` passes `&mut Board` to `get_best_move`; engine applies/undos moves during search without permanent changes. No async/channels; pure synchronous calls.
- **No External Services**: Fully local; `openings_sheet.csv` is static. For extensions, integrate UCI protocol using `pleco`'s engine traits.
- Example integration in `main.rs` (game loop after opening):
  ```rust
  while !board.checkmate() && !board.stalemate() {
      let best_move = engine::get_best_move(&mut board, engine::MAX_DEPTH);
      if best_move.is_null() { break; }
      println!("Best move: {:?} (turn: {:?})", best_move, board.turn());
      board.apply_move(best_move);
      println!("Board after move:\n{:?}", board);
  }
  ```

When contributing:
- Adhere to `pleco` idioms (e.g., `BitMove::null()` for invalid moves).
- Enhance search incrementally: Improve TT replacement, add endgame PSQT, integrate PV extraction for better ordering.
- Validate: Run `cargo run` multiple times; verify consistent playouts, check for legal moves only, monitor search depth impact on game length.

Is there anything unclear or incomplete in these instructions? Let me know how to refine them.