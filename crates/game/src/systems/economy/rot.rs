#![allow(dead_code)]

use super::{MoneyCents, RotCfg};

pub fn convert_rot_to_debt(rot_u16: u16, cfg: &RotCfg) -> (u16, MoneyCents) {
    let clamped = rot_u16.clamp(cfg.rot_floor, cfg.rot_ceiling);
    if clamped <= cfg.rot_floor || cfg.conversion_chunk == 0 {
        return (clamped, MoneyCents::ZERO);
    }

    let convertible = clamped - cfg.rot_floor;
    let chunks = convertible / cfg.conversion_chunk;
    if chunks == 0 {
        return (clamped, MoneyCents::ZERO);
    }

    let rot_after = clamped - chunks * cfg.conversion_chunk;
    let debt_delta = MoneyCents::from_i128_clamped(
        i128::from(cfg.debt_per_chunk_cents) * i128::from(chunks),
    );
    (rot_after, debt_delta)
}
