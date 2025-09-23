use crate::{
    geometry::{CellId, Coord2D},
    geometry::BoardGeometry,
    EngineError, GameState, MoveDraft, Tile,
    CrosswordRules, Rules, ScoreBreakdown, ValidatedMove,
};

#[derive(Debug, Clone)]
pub enum Action {
    Place { x: i32, y: i32, kind_id: String, mark: Option<String> },
    Stack { x: i32, y: i32, kind_id: String, mark: Option<String> },
    SwapRack { give: Vec<String> },
    RotateTile { x: i32, y: i32 },
    SlideGroup { cells: Vec<(i32, i32)>, dx: i32, dy: i32 },
    Custom(String, serde_json::Value),
}

#[derive(Debug, Clone, Default)]
pub struct UserMove { pub actions: Vec<Action> }

pub trait RulePlugin {
    fn name(&self) -> &str { "plugin" }
    fn pre_validate(&self, _state: &GameState, _mv: &mut UserMove) -> Result<(), EngineError> { Ok(()) }
    fn validate(&self, _state: &GameState, _mv: &UserMove) -> Result<(), EngineError> { Ok(()) }
    fn to_draft(&self, _state: &GameState, _mv: &UserMove) -> Option<MoveDraft> { None }
    fn modify_score(&self, _state: &GameState, _v: &ValidatedMove, _sc: &mut ScoreBreakdown) {}
    fn commit(&self, _state: &mut GameState, _v: &ValidatedMove, _sc: &ScoreBreakdown) -> Result<(), EngineError> { Ok(()) }
}

#[derive(Debug, Default)]
pub struct BasicActionsPlugin;
impl RulePlugin for BasicActionsPlugin {
    fn name(&self) -> &str { "basic_actions" }
    fn pre_validate(&self, _state: &GameState, mv: &mut UserMove) -> Result<(), EngineError> {
        for a in &mv.actions {
            match a {
                Action::Place { .. } | Action::Stack { .. } | Action::Custom(_, _) => {}
                _ => return Err(EngineError::Config("unsupported action")),
            }
        }
        Ok(())
    }
    fn to_draft(&self, state: &GameState, mv: &UserMove) -> Option<MoveDraft> {
        let mut placements: Vec<(CellId, Tile)> = Vec::new();
        for a in &mv.actions {
            match a {
                Action::Place { x, y, kind_id, mark } | Action::Stack { x, y, kind_id, mark } => {
                    let id = state.board.geom.to_cell_id(Coord2D { x: *x, y: *y })?;
                    placements.push((id, Tile { kind_id: kind_id.clone(), mark: mark.clone() }));
                }
                Action::Custom(_, _) => {}
                _ => {}
            }
        }
        if placements.is_empty() { None } else { Some(MoveDraft { placements }) }
    }
}

#[derive(Debug)]
pub struct ScoreBonusPlugin { pub bonus: i32 }
impl RulePlugin for ScoreBonusPlugin {
    fn name(&self) -> &str { "score_bonus" }
    fn modify_score(&self, _state: &GameState, _v: &ValidatedMove, sc: &mut ScoreBreakdown) { sc.total += self.bonus; }
}

pub struct PluginRules { pub base: CrosswordRules, pub plugins: Vec<Box<dyn RulePlugin + Send + Sync>> }

impl PluginRules {
    pub fn new(base: CrosswordRules, plugins: Vec<Box<dyn RulePlugin + Send + Sync>>) -> Self { Self { base, plugins } }
    pub fn validate_user_move(&self, state: &GameState, mut mv: UserMove) -> Result<ValidatedMove, EngineError> {
        for p in &self.plugins { p.pre_validate(state, &mut mv)?; }
        for p in &self.plugins { p.validate(state, &mv)?; }
        let mut draft: Option<MoveDraft> = None;
        for p in &self.plugins { if let Some(d) = p.to_draft(state, &mv) { draft = Some(d); } }
        let draft = draft.ok_or(EngineError::Config("no draft produced by plugins"))?;
        self.base.validate(state, &draft)
    }
    pub fn score_user_move(&self, state: &GameState, v: &ValidatedMove) -> ScoreBreakdown {
        let mut sc = self.base.score(state, v);
        for p in &self.plugins { p.modify_score(state, v, &mut sc); }
        sc
    }
    pub fn commit_user_move(&self, state: &mut GameState, v: ValidatedMove, sc: &ScoreBreakdown) -> Result<(), EngineError> {
        self.base.commit(state, v.clone(), sc)?;
        for p in &self.plugins { p.commit(state, &v, sc)?; }
        Ok(())
    }
}

impl Rules for PluginRules {
    fn validate(&self, state: &GameState, draft: &MoveDraft) -> Result<ValidatedMove, EngineError> { self.base.validate(state, draft) }
    fn score(&self, state: &GameState, mv: &ValidatedMove) -> ScoreBreakdown { self.base.score(state, mv) }
    fn commit(&self, state: &mut GameState, mv: ValidatedMove, score: &ScoreBreakdown) -> Result<(), EngineError> { self.base.commit(state, mv, score) }
}
