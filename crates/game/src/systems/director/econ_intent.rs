use bevy::prelude::Resource;

/// Accumulates pending economic deltas to be applied after a mission resolves.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EconIntent {
    pub pending_pp_delta: i16,
    pub pending_basis_overlay_bp: i16,
}

impl EconIntent {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
