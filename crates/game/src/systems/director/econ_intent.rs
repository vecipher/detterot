use bevy::prelude::Resource;

#[derive(Resource, Default, Debug)]
pub struct EconIntent {
    pub pending_pp_delta: i16,
    pub pending_basis_overlay_bp: i16,
}

impl EconIntent {
    pub fn apply(&mut self, pp_delta: i16, basis_delta: i16) {
        self.pending_pp_delta = self.pending_pp_delta.saturating_add(pp_delta);
        self.pending_basis_overlay_bp = self.pending_basis_overlay_bp.saturating_add(basis_delta);
    }
}
