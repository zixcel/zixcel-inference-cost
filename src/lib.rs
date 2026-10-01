#![forbid(unsafe_code)]
#![doc = "Deterministic provider-neutral inference usage and cost estimation."]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const USAGE_SCHEMA_V1: &str = "zixcel://inference-cost/token-usage/v1";
pub const RATE_CARD_SCHEMA_V1: &str = "zixcel://inference-cost/rate-card/v1";
pub const ESTIMATE_SCHEMA_V1: &str = "zixcel://inference-cost/estimate/v1";

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_write_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
}

impl TokenUsage {
    /// Creates a usage record, rejecting an impossible cached-input count.
    ///
    /// # Errors
    ///
    /// Returns [`CostError::CachedInputExceedsInput`] when cached input is
    /// greater than total input.
    pub fn new(
        input_tokens: u64,
        cached_input_tokens: u64,
        cache_write_input_tokens: u64,
        output_tokens: u64,
        reasoning_output_tokens: u64,
    ) -> Result<Self, CostError> {
        if cached_input_tokens > input_tokens {
            return Err(CostError::CachedInputExceedsInput);
        }
        Ok(Self {
            input_tokens,
            cached_input_tokens,
            cache_write_input_tokens,
            output_tokens,
            reasoning_output_tokens,
        })
    }

    #[must_use]
    pub fn digest(&self) -> String {
        digest_json(self)
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CostRateCard {
    pub schema: String,
    pub unit: String,
    pub source: String,
    pub as_of: String,
    pub input_micro_units_per_token: u64,
    pub cached_input_micro_units_per_token: u64,
    pub cache_write_input_micro_units_per_token: u64,
    pub output_micro_units_per_token: u64,
    pub reasoning_output_micro_units_per_token: u64,
}

impl CostRateCard {
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn per_token(
        unit: impl Into<String>,
        source: impl Into<String>,
        as_of: impl Into<String>,
        input: u64,
        cached_input: u64,
        cache_write_input: u64,
        output: u64,
        reasoning_output: u64,
    ) -> Self {
        Self {
            schema: RATE_CARD_SCHEMA_V1.to_owned(),
            unit: unit.into(),
            source: source.into(),
            as_of: as_of.into(),
            input_micro_units_per_token: input,
            cached_input_micro_units_per_token: cached_input,
            cache_write_input_micro_units_per_token: cache_write_input,
            output_micro_units_per_token: output,
            reasoning_output_micro_units_per_token: reasoning_output,
        }
    }

    /// Validates the identity and provenance fields of the rate card.
    ///
    /// # Errors
    ///
    /// Returns [`CostError::InvalidRateCard`] for an unsupported schema or an
    /// empty unit, source, or timestamp.
    pub fn validate(&self) -> Result<(), CostError> {
        if self.schema != RATE_CARD_SCHEMA_V1
            || self.unit.trim().is_empty()
            || self.source.trim().is_empty()
            || self.as_of.trim().is_empty()
        {
            return Err(CostError::InvalidRateCard);
        }
        Ok(())
    }

    #[must_use]
    pub fn digest(&self) -> String {
        digest_json(self)
    }
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CostEstimate {
    pub schema: String,
    pub unit: String,
    pub usage_digest: String,
    pub rate_card_digest: String,
    pub input_micro_units: u64,
    pub cached_input_micro_units: u64,
    pub cache_write_input_micro_units: u64,
    pub output_micro_units: u64,
    pub reasoning_output_micro_units: u64,
    pub total_micro_units: u64,
    pub billing_authoritative: bool,
}

impl CostEstimate {
    #[must_use]
    pub const fn total_micro_units(&self) -> u64 {
        self.total_micro_units
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CostError {
    CachedInputExceedsInput,
    InvalidRateCard,
    ArithmeticOverflow,
    Serialization,
}

impl std::fmt::Display for CostError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::CachedInputExceedsInput => "cached input tokens exceed input tokens",
            Self::InvalidRateCard => "invalid rate card",
            Self::ArithmeticOverflow => "cost arithmetic overflow",
            Self::Serialization => "deterministic serialization failed",
        })
    }
}

impl std::error::Error for CostError {}

/// Estimates provider usage using the supplied integer rate card.
///
/// # Errors
///
/// Returns an error when the rate card is invalid or checked arithmetic would
/// overflow.
pub fn estimate(usage: &TokenUsage, rates: &CostRateCard) -> Result<CostEstimate, CostError> {
    rates.validate()?;
    // Provider usage reports `input_tokens` as the total input, while cached
    // input is a billable subset with its own rate. Charge only the uncached
    // remainder at the ordinary input rate; charging the total would count
    // cached tokens twice.
    let uncached_input_tokens = usage
        .input_tokens
        .checked_sub(usage.cached_input_tokens)
        .ok_or(CostError::CachedInputExceedsInput)?;
    let input = multiply(uncached_input_tokens, rates.input_micro_units_per_token)?;
    let cached = multiply(
        usage.cached_input_tokens,
        rates.cached_input_micro_units_per_token,
    )?;
    let cache_write = multiply(
        usage.cache_write_input_tokens,
        rates.cache_write_input_micro_units_per_token,
    )?;
    let output = multiply(usage.output_tokens, rates.output_micro_units_per_token)?;
    let reasoning_output = multiply(
        usage.reasoning_output_tokens,
        rates.reasoning_output_micro_units_per_token,
    )?;
    let total = [input, cached, cache_write, output, reasoning_output]
        .into_iter()
        .try_fold(0_u64, u64::checked_add)
        .ok_or(CostError::ArithmeticOverflow)?;

    Ok(CostEstimate {
        schema: ESTIMATE_SCHEMA_V1.to_owned(),
        unit: rates.unit.clone(),
        usage_digest: usage.digest(),
        rate_card_digest: rates.digest(),
        input_micro_units: input,
        cached_input_micro_units: cached,
        cache_write_input_micro_units: cache_write,
        output_micro_units: output,
        reasoning_output_micro_units: reasoning_output,
        total_micro_units: total,
        billing_authoritative: false,
    })
}

/// Conservative byte-based estimate used only when a provider did not return usage.
#[must_use]
pub const fn approximate_tokens_from_bytes(bytes: usize) -> usize {
    bytes.saturating_add(3) / 4
}

#[must_use]
pub const fn approximate_token_count(text: &str) -> usize {
    approximate_tokens_from_bytes(text.len())
}

#[must_use]
pub const fn approximate_bytes_for_tokens(tokens: usize) -> usize {
    tokens.saturating_mul(4)
}

#[must_use]
pub const fn approximate_tokens_from_byte_count(bytes: usize) -> u64 {
    approximate_tokens_from_bytes(bytes) as u64
}

fn multiply(left: u64, right: u64) -> Result<u64, CostError> {
    left.checked_mul(right).ok_or(CostError::ArithmeticOverflow)
}

fn digest_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_each_usage_component_without_floating_point() {
        let usage = TokenUsage::new(1_000, 200, 0, 120, 40).expect("usage");
        let rates =
            CostRateCard::per_token("micro-unit", "model-rate", "2026-09-04", 2, 1, 3, 4, 5);
        let result = estimate(&usage, &rates).expect("estimate");
        assert_eq!(result.input_micro_units, 1_600);
        assert_eq!(result.cached_input_micro_units, 200);
        assert_eq!(result.total_micro_units(), 2_480);
        assert!(!result.billing_authoritative);
    }

    #[test]
    fn rejects_inconsistent_cached_usage() {
        assert_eq!(
            TokenUsage::new(1, 2, 0, 0, 0),
            Err(CostError::CachedInputExceedsInput)
        );
    }

    #[test]
    fn approximation_is_deterministic() {
        assert_eq!(approximate_tokens_from_bytes(5), 2);
        assert_eq!(approximate_bytes_for_tokens(2), 8);
    }

    #[test]
    fn rejects_checked_arithmetic_overflow() {
        let usage = TokenUsage::new(2, 0, 0, 0, 0).expect("usage");
        let rates = CostRateCard::per_token(
            "micro-unit",
            "model-rate",
            "2026-09-04",
            u64::MAX,
            0,
            0,
            0,
            0,
        );
        assert_eq!(estimate(&usage, &rates), Err(CostError::ArithmeticOverflow));
    }
}
