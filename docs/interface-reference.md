# zixcel-inference-cost interface reference

Use the [usage guide](getting-started.md) for the first steps. This reference preserves the current interface details and operational limits. Run command examples from the repository root, after preparing the exact declared dependencies and registered configuration.

## Usage

```rust
use zixcel_inference_cost::{CostRateCard, TokenUsage, estimate};

let usage = TokenUsage::new(1_000, 200, 0, 120, 40)?;
let rates = CostRateCard::per_token("micro-unit", "model-rate", "2026-09-04", 2, 1, 3, 4, 5);
let result = estimate(&usage, &rates)?;
// 800 uncached input × 2 + 200 cached × 1 + 120 output × 4 + 40 reasoning × 5.
assert_eq!(result.total_micro_units(), 2_480);
# Ok::<(), zixcel_inference_cost::CostError>(())
```

The rates are informational estimates and must not silently control model
selection. Provider billing remains authoritative at the provider boundary.
