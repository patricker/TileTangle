use crate::movegen::CandidateMove;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatedMove {
    pub candidate: CandidateMove,
    pub rack_leave: i32,
    pub board_equity: i32,
    pub endgame_penalty: i32,
    pub total: i32,
}

