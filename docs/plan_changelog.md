# Planning changelog

## v1.0.3 (M3 trading stack)
- Trading UI, pricing view, cargo, and save v1.1 integration landed.
- Deterministic trading replay goldens added to CI (macOS + Ubuntu).
- Trading config hashes:
  - `assets/trading/commodities.toml` — `acfae862242a4f11573143ee00b25f47dae2fcf4a9061e92001f5dd386d09a8b`
  - `assets/trading/config.toml` — `cfa91ae88482054e3676435d183d63de583aeee3e10414559bc0f6387f945e39`

## v1.0.2 (M1 economy milestones)
- Locked in macro/micro money supply goals for milestone 1.
- Landed deterministic interest/basis adjustments in the game economy systems.
- Synced economy save/load schemas with the simulation harness for reproducible runs.
