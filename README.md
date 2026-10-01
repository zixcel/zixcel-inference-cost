# zixcel-inference-cost

Provider-neutral, local-only inference usage and cost estimation.

This package does not discover prices, contact a provider, resolve credentials,
route a model, or perform billing. The caller supplies measured usage and a
versioned rate card. All arithmetic uses integer micro-units and checked
operations. A result contains digests for both the usage and the rate card so
that a projection can be reproduced without retaining the original request.

Callers reference the estimator as a versioned dependency and own execution,
authorization, transport and scheduling integration.

The JSON contracts are in [`schemas/`](schemas/). The `$id` values use the
package-owned `zixcel://inference-cost/schema/...` namespace and are separate
from transport protocol identifiers.

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

## Package integration

The package is an independently consumable unit. Callers reference its documented
interface through a versioned dependency and own application-specific composition
and integration.

## Distribution license

Apache-2.0. Copyright 2026 HAT Inc. See [LICENSE](LICENSE) and [NOTICE](NOTICE). Earlier license files and third-party terms remain applicable to their respective portions.
