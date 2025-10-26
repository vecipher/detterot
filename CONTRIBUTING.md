## Etiquette
- Feature branches; merge via PR with green CI.
- No TODOs without a linked issue.
- Changes to economy/saves/rulepacks must update tests and changelog.

## Performance budgets
- Gameplay ≤ 6 ms CPU; GPU ≤ 8/16/33 ms for 120/60/30 FPS tiers.

## Determinism
- Fixed timestep; DetRng only; stable system sets/order.

## Formatting
- Run `cargo fmt --all` locally before pushing to avoid CI failures on the formatting check.
