# Using zixcel-inference-cost

Calculate an inference-cost estimate from explicit usage and price inputs.

## Before you start

Prices and usage are caller inputs. The package does not retrieve prices, bill an account or purchase capacity.

## First steps

Run from the repository root:

```sh
cargo test --locked
```

## How to assess the result

- Validate supplied usage and rates.
- Return reproducible calculated costs.

A passing source-level check establishes only what that check observes. Keep missing configuration, unavailable services and unverified deployment paths visible.

## Continue reading

[Repository overview](../README.md)
