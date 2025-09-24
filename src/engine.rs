use pleco::{Board, BitMove, Player, PieceType, SQ};
use std::collections::HashMap;

pub const MAX_DEPTH: u8 = 5;
pub const INFINITY: i32 = 1000000;
pub const MAX_PLY: usize = 64;

#[derive(Clone, Copy)]
pub struct TTEntry {
    pub score: i32,
    pub depth: u8,
    pub flag: u8, // 0: exact, 1: lower, 2: upper
}

pub struct CustomTT {
    pub table: HashMap<u64, TTEntry>,
}

impl CustomTT {
    pub fn new() -> Self {
        CustomTT { table: HashMap::new() }
    }

    pub fn probe(&mut self, hash: u64, depth: u8, alpha: i32, beta: i32) -> Option<i32> {
        if let Some(entry) = self.table.get(&hash) {
            if entry.depth >= depth {
                match entry.flag {
                    0 => Some(entry.score),
                    1 => if entry.score >= beta { Some(entry.score) } else { None },
                    2 => if entry.score <= alpha { Some(entry.score) } else { None }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn store(&mut self, hash: u64, depth: u8, score: i32, flag: u8) {
        self.table.insert(hash, TTEntry { score, depth, flag });
    }
}

// Helper functions
fn file_char(file_idx: u8) -> char {
    (b'a' + file_idx) as char
}

fn rank_char(rank_idx: u8) -> char {
    (b'1' + rank_idx) as char
}

fn piece_char(pt: PieceType) -> char {
    match pt {
        PieceType::P => ' ',
        PieceType::N => 'N',
        PieceType::B => 'B',
        PieceType::R => 'R',
        PieceType::Q => 'Q',
        PieceType::K => 'K',
        PieceType::None | PieceType::All => '?',
    }
}

pub fn bitmove_to_san(board: &Board, mv: BitMove) -> String {
    if mv.is_null() {
        return String::new();
    }

    let from_sq = mv.get_src();
    let to_sq = mv.get_dest();
    let moved_pt = board.moved_piece(mv).type_of();

    let mut san = String::new();

    // Piece symbol
    let pc = piece_char(moved_pt);
    if pc != ' ' {
        san.push(pc);
    }

    // Castling
    if mv.is_king_castle() {
        return "O-O".to_string();
    }
    if mv.is_queen_castle() {
        return "O-O-O".to_string();
    }

    // Disambiguation (basic)
    let mut disamb = String::new();
    let moves = board.generate_moves();
    let mut count_same = 0;
    for other_mv in moves.iter() {
        if board.legal_move(*other_mv) && other_mv.get_dest() == to_sq && board.moved_piece(*other_mv).type_of() == moved_pt && *other_mv != mv {
            count_same += 1;
        }
    }
    if count_same > 0 {
        let from_file_idx = pleco::core::file_idx_of_sq(from_sq.0);
        disamb.push(file_char(from_file_idx));
    }
    san.push_str(&disamb);

    // Capture
    if mv.is_capture() {
        if moved_pt == PieceType::P {
            let from_file_idx = pleco::core::file_idx_of_sq(from_sq.0);
            san.push(file_char(from_file_idx));
        }
        san.push('x');
    }

    // Destination
    let to_file_idx = pleco::core::file_idx_of_sq(to_sq.0);
    let to_rank_idx = pleco::core::rank_idx_of_sq(to_sq.0);
    san.push(file_char(to_file_idx));
    san.push(rank_char(to_rank_idx));

    // Promotion (basic: pawn to last rank, default Q)
    let is_white = board.turn() == Player::White;
    let last_rank_idx = if is_white { 7u8 } else { 0u8 };
    if moved_pt == PieceType::P && to_rank_idx == last_rank_idx {
        san.push('=');
        san.push('Q');
    }

    // Check suffix
    let mut temp_board = board.shallow_clone();
    temp_board.apply_move(mv);
    if temp_board.in_check() {
        san.push('+');
    }
    if temp_board.checkmate() {
        san.push('#');
    }

    san
}

pub fn san_to_bitmove(board: &Board, san: &str) -> Result<BitMove, String> {
    let lowered = san.trim().to_lowercase();
    let clean_san = lowered.trim_end_matches(|c| c == '+' || c == '#');

    // Castling
    let is_white = board.turn() == Player::White;
    let moves = board.generate_moves();
    if clean_san == "o-o" {
        for mv in moves.iter() {
            if board.legal_move(*mv) && mv.is_king_castle() {
                return Ok(*mv);
            }
        }
    } else if clean_san == "o-o-o" {
        for mv in moves.iter() {
            if board.legal_move(*mv) && mv.is_queen_castle() {
                return Ok(*mv);
            }
        }
    }

    // General match
    for mv in moves.iter() {
        if board.legal_move(*mv) {
            let mv_san = bitmove_to_san(board, *mv).to_lowercase();
            if mv_san == clean_san {
                return Ok(*mv);
            }
        }
    }

    Err(format!("No matching move for SAN: {}", san))
}

// Fix evaluate_board parens
pub fn evaluate_board(board: &Board) -> i32 {
    let mut score = 0i32;
    for i in 0u8..64 {
        let sq = SQ(i);
        let p = board.piece_at_sq(sq);
        if let Some(pl) = p.player() {
            let pt = p.type_of();
            let material_value = match pt {
                PieceType::P => 100,
                PieceType::N => 320,
                PieceType::B => 330,
                PieceType::R => 500,
                PieceType::Q => 900,
                PieceType::K => 20000,
                _ => 0,
            };
            let psqt_index = if pl == Player::White { sq.0 as usize } else { 63 - sq.0 as usize };
            let psqt_value = match pt {
                PieceType::P => PSQT_PAWN_MG[psqt_index],
                PieceType::N => PSQT_KNIGHT_MG[psqt_index],
                PieceType::B => PSQT_BISHOP_MG[psqt_index],
                PieceType::R => PSQT_ROOK_MG[psqt_index],
                PieceType::Q => PSQT_QUEEN_MG[psqt_index],
                PieceType::K => PSQT_KING_MG[psqt_index],
                _ => 0,
            };
            let total_value = material_value as i16 + psqt_value;
            if pl == Player::White {
                score += total_value as i32;
            } else {
                score -= total_value as i32;
            }
        }
    }
    if board.turn() == Player::White { score } else { -score }
}

// PSQT constants...
pub const PSQT_PAWN_MG: [i16; 64] = [
    0, 0, 0, 0, 0, 0, 0, 0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
    5, 5, 10, 25, 25, 10, 5, 5,
    0, 0, 0, 20, 20, 0, 0, 0,
    5, -5, -10, 0, 0, -10, -5, 5,
    5, 10, 10, -20, -20, 10, 10, 5,
    0, 0, 0, 0, 0, 0, 0, 0,
];

pub const PSQT_KNIGHT_MG: [i16; 64] = [
    -50, -40, -30, -30, -30, -30, -40, -50,
    -40, -20, 0, 5, 5, 0, -20, -40,
    -30, 5, 10, 15, 15, 10, 5, -30,
    -30, 0, 15, 20, 20, 15, 0, -30,
    -30, 5, 15, 20, 20, 15, 5, -30,
    -30, 0, 10, 15, 15, 10, 0, -30,
    -40, -20, 0, 0, 0, 0, -20, -40,
    -50, -40, -30, -30, -30, -30, -40, -50,
];

pub const PSQT_BISHOP_MG: [i16; 64] = [
    -20, -10, -10, -10, -10, -10, -10, -20,
    -10, 5, 0, 0, 0, 0, 5, -10,
    -10, 10, 10, 10, 10, 10, 10, -10,
    -10, 0, 10, 10, 10, 10, 0, -10,
    -10, 5, 5, 10, 10, 5, 5, -10,
    -10, 0, 5, 10, 10, 5, 0, -10,
    -10, 0, 0, 0, 0, 0, 0, -10,
    -20, -10, -10, -10, -10, -10, -10, -20,
];

pub const PSQT_ROOK_MG: [i16; 64] = [
    0, 0, 0, 5, 5, 0, 0, 0,
    -5, 0, 0, 0, 0, 0, 0, -5,
    -5, 0, 0, 0, 0, 0, 0, -5,
    -5, 0, 0, 0, 0, 0, 0, -5,
    -5, 0, 0, 0, 0, 0, 0, -5,
    -5, 0, 0, 0, 0, 0, 0, -5,
    5, 10, 10, 10, 10, 10, 10, 5,
    0, 0, 0, 0, 0, 0, 0, 0,
];

pub const PSQT_QUEEN_MG: [i16; 64] = [
    -20, -10, -10, -5, -5, -10, -10, -20,
    -10, 0, 5, 0, 0, 0, 0, -10,
    -10, 5, 5, 5, 5, 5, 0, -10,
    0, 0, 5, 5, 5, 5, 0, -5,
    -5, 0, 5, 5, 5, 5, 0, -5,
    -10, 0, 5, 5, 5, 5, 0, -10,
    -10, 0, 0, 0, 0, 0, 0, -10,
    -20, -10, -10, -5, -5, -10, -10, -20,
];

pub const PSQT_KING_MG: [i16; 64] = [
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -30, -40, -40, -50, -50, -40, -40, -30,
    -20, -30, -30, -40, -40, -30, -30, -20,
    -10, -20, -20, -20, -20, -20, -20, -10,
    20, 20, 0, 0, 0, 0, 20, 20,
    20, 30, 10, 0, 0, 10, 30, 20,
];

static mut KILLERS: [[BitMove; 2]; MAX_PLY] = [[BitMove::null(); 2]; MAX_PLY];
static mut HISTORY: [[i32; 64]; 6] = [[0i32; 64]; 6];

pub fn score_move(m: BitMove, board: &Board) -> i32 {
    if board.is_capture(m) {
        // MVV-LVA using pleco values
        let victim_pt = board.captured_piece(m);
        let attacker_pt = board.moved_piece(m).type_of();
        let mvv = match victim_pt {
            PieceType::P => 100,
            PieceType::N => 320,
            PieceType::B => 330,
            PieceType::R => 500,
            PieceType::Q => 900,
            PieceType::K => 20000,
            _ => 0,
        };
        let lva = match attacker_pt {
            PieceType::P => 100,
            PieceType::N => 320,
            PieceType::B => 330,
            PieceType::R => 500,
            PieceType::Q => 900,
            PieceType::K => 20000,
            _ => 0,
        };
        10000 + mvv * 100 - lva
    } else if m == unsafe { KILLERS[0][0] } || m == unsafe { KILLERS[0][1] } {
        return 9000;
    } else {
        let pt_idx = match board.moved_piece(m).type_of() {
            PieceType::P => 0,
            PieceType::N => 1,
            PieceType::B => 2,
            PieceType::R => 3,
            PieceType::Q => 4,
            PieceType::K => 5,
            _ => 0,
        };
        let sq = m.get_dest();
        unsafe { HISTORY[pt_idx][sq.0 as usize] + 1000 }
    }
}

pub fn order_moves(moves: &mut Vec<BitMove>, board: &Board) { // Use slice for efficiency
    moves.sort_by_key(|&m| std::cmp::Reverse(score_move(m, board)));
}

pub fn get_best_move(board: &mut Board, max_depth: u8) -> BitMove {
    let mut tt = CustomTT::new();
    let maximizing = board.turn() == Player::White;
    let mut best_move = BitMove::null();

    let moves: Vec<BitMove> = board.generate_pseudolegal_moves()
        .iter()
        .filter(|&&m| board.legal_move(m))
        .cloned()
        .collect();
    if moves.is_empty() {
        return BitMove::null();
    }

    for d in 1..=max_depth {
        let mut current_best = BitMove::null();
        let mut current_value = if maximizing { -INFINITY } else { INFINITY };

        let mut moves_vec: Vec<BitMove> = moves.iter().cloned().collect();
        order_moves(&mut moves_vec, board);

        for m in moves_vec {
            board.apply_move(m);
            let value = minimax(board, d - 1, -INFINITY, INFINITY, !maximizing, &mut tt);
            board.undo_move();

            if (maximizing && value > current_value) || (!maximizing && value < current_value) {
                current_value = value;
                current_best = m;
            }
        }

        if current_best.is_null() {
            break;
        }

        best_move = current_best;
    }

    best_move
}

pub fn minimax(board: &mut Board, depth: u8, mut alpha: i32, mut beta: i32, maximizing: bool, tt: &mut CustomTT) -> i32 {
    let hash = board.zobrist();

    if let Some(score) = tt.probe(hash, depth, alpha, beta) {
        return score;
    }

    if board.checkmate() {
        let score = if board.turn() == Player::White { -INFINITY } else { INFINITY };
        tt.store(hash, depth, score, 0);
        return score;
    }
    if board.stalemate() {
        tt.store(hash, depth, 0, 0);
        return 0;
    }

    if depth == 0 {
        let score = quiescence(board, alpha, beta, tt);
        tt.store(hash, 0, score, 0);
        return score;
    }

    // Null-move pruning
    if depth >= 3 && !maximizing && alpha < beta && !board.in_check() {
        unsafe { board.apply_null_move(); }
        let null_score = -minimax(board, depth - 3, -beta, -alpha, true, tt);
        unsafe { board.undo_null_move(); }
        if null_score >= beta {
            tt.store(hash, depth, null_score, 2);
            return null_score;
        }
    }

    let mut moves_vec: Vec<BitMove> = board.generate_pseudolegal_moves()
        .iter()
        .filter(|&&m| board.legal_move(m))
        .cloned()
        .collect();
    order_moves(&mut moves_vec, board);

    let mut score;
    let mut flag = 0u8;
    if maximizing {
        score = -INFINITY;
        for m in moves_vec {
            let moved_pt = board.moved_piece(m).type_of();
            board.apply_move(m);
            let eval = minimax(board, depth - 1, alpha, beta, false, tt);
            board.undo_move();
            score = score.max(eval);
            alpha = alpha.max(eval);
            if beta <= alpha {
                flag = 1;
                unsafe {
                    let ply = board.ply() as usize % MAX_PLY;
                    if KILLERS[ply][0].is_null() {
                        KILLERS[ply][0] = m;
                    } else if KILLERS[ply][1].is_null() {
                        KILLERS[ply][1] = m;
                    }
                    let pt_idx = match moved_pt {
                        PieceType::P => 0,
                        PieceType::N => 1,
                        PieceType::B => 2,
                        PieceType::R => 3,
                        PieceType::Q => 4,
                        PieceType::K => 5,
                        _ => 0,
                    };
                    let sq = m.get_dest();
                    HISTORY[pt_idx][sq.0 as usize] += depth as i32 * depth as i32;
                }
                break;
            }
        }
    } else {
        score = INFINITY;
        for m in moves_vec {
            let moved_pt = board.moved_piece(m).type_of();
            board.apply_move(m);
            let eval = minimax(board, depth - 1, alpha, beta, true, tt);
            board.undo_move();
            score = score.min(eval);
            beta = beta.min(eval);
            if beta <= alpha {
                flag = 2;
                unsafe {
                    let ply = board.ply() as usize % MAX_PLY;
                    if KILLERS[ply][0].is_null() {
                        KILLERS[ply][0] = m;
                    } else if KILLERS[ply][1].is_null() {
                        KILLERS[ply][1] = m;
                    }
                    let pt_idx = match moved_pt {
                        PieceType::P => 0,
                        PieceType::N => 1,
                        PieceType::B => 2,
                        PieceType::R => 3,
                        PieceType::Q => 4,
                        PieceType::K => 5,
                        _ => 0,
                    };
                    let sq = m.get_dest();
                    HISTORY[pt_idx][sq.0 as usize] += depth as i32 * depth as i32;
                }
                break;
            }
        }
    }

    tt.store(hash, depth, score, flag);
    score
}

fn quiescence(board: &mut Board, mut alpha: i32, beta: i32, tt: &mut CustomTT) -> i32 {
    let hash = board.zobrist();
    if let Some(score) = tt.probe(hash, 0, alpha, beta) {
        return score;
    }

    let stand_pat = evaluate_board(board);
    if stand_pat >= beta {
        return beta;
    }
    if alpha < stand_pat {
        alpha = stand_pat;
    }

    let all_moves = board.generate_pseudolegal_moves();
    let mut moves_vec: Vec<BitMove> = all_moves
        .iter()
        .filter(|&&m| board.is_capture(m) && board.legal_move(m))
        .cloned()
        .collect();
    order_moves(&mut moves_vec, board);

    let mut score = stand_pat;
    for m in moves_vec {
        if !board.see_ge(m, 0) { continue; }
        board.apply_move(m);
        let eval = -quiescence(board, -beta, -alpha, tt);
        board.undo_move();
        if eval >= beta {
            tt.store(hash, 0, beta, 2);
            return beta;
        }
        if eval > alpha {
            alpha = eval;
        }
        score = score.max(eval);
    }
    tt.store(hash, 0, alpha, 0);
    alpha
}
