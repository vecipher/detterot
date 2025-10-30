# Trading systems guide

## Price policy and drivers
- Trades quote prices through the deterministic `PriceView`, which feeds the
  current daily index (DI) and hub basis multipliers into `compute_price` while
  keeping all math in integer `MoneyCents` units (no floats in the price path).
  【F:crates/game/src/systems/trading/pricing_vm.rs†L8-L47】【F:crates/game/src/systems/economy/pricing.rs†L9-L33】
- Driver inputs (DI + basis) are clamped to the rulepack's multiplier bounds
  before applying the integer multiplier to the base price, preventing runaway
  quotes when the economy spikes.【F:crates/game/src/systems/economy/pricing.rs†L14-L23】

## Rounding guarantees
- Quotes scale the base price into milli-cents, apply banker's rounding on
  half-cent ties, and then perform a final downward clamp to ensure no residual
  fractions survive beyond a cent.【F:crates/game/src/systems/economy/pricing.rs†L21-L34】【F:crates/game/src/systems/economy/rounding.rs†L5-L19】
- All intermediate products go through `MoneyCents::from_i128_clamped`, so
  integer overflow collapses into the signed 64-bit bounds instead of wrapping
  or panicking.【F:crates/game/src/systems/economy/money.rs†L11-L28】【F:crates/game/src/systems/trading/engine.rs†L96-L109】

## Transaction fees
- Executed trades compute gross notional via saturating integer multiplication
  and apply the transaction fee basis points using the same integer scaling,
  guaranteeing fee math matches price rounding and never introduces floats.【F:crates/game/src/systems/trading/engine.rs†L80-L109】
- Fee calculations share the clamped arithmetic path, so extreme wallet sizes or
  fee settings cannot overflow the accumulator.【F:crates/game/src/systems/trading/engine.rs†L96-L109】

## Capacity rules
- Buy orders enforce three independent limits before any units fill: remaining
  cargo volume, remaining cargo mass, and wallet affordability under the fee
  schedule. The engine executes the minimum of those caps, so breaching any one
  limit short-circuits the fill.【F:crates/game/src/systems/trading/engine.rs†L53-L85】
- Sell orders are capped by existing inventory units, preventing negative cargo
  balances even if the caller requests more than the manifest holds.【F:crates/game/src/systems/trading/engine.rs†L86-L94】
- After execution, the engine applies saturating updates to cargo capacity usage
  and wallet balances so post-trade state stays within the configured bounds.【F:crates/game/src/systems/trading/engine.rs†L110-L152】

## Wallet affordability search
- Wallet checks use a binary search against the integer fee-inclusive trade
  cost, ensuring the affordability gate stays deterministic and converges without
  float math. Costs share the same clamped multiplication and fee logic as final
  execution.【F:crates/game/src/systems/trading/engine.rs†L114-L199】
