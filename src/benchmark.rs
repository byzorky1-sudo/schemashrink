use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkStats {
    pub total_tools: usize,
    pub original_bytes: usize,
    pub compressed_bytes: usize,
    pub original_tokens_est: usize,
    pub compressed_tokens_est: usize,
    pub tokens_saved_per_turn: usize,
    pub monthly_tokens_saved: u64,
    pub monthly_dollar_savings: f64,
}

pub struct BenchmarkRunner;

impl BenchmarkRunner {
    pub fn calculate_savings(
        original_tokens: usize,
        compressed_tokens: usize,
        monthly_requests: u64,
        price_per_million_tokens: f64,
    ) -> BenchmarkStats {
        let tokens_saved_per_turn = original_tokens.saturating_sub(compressed_tokens);
        let monthly_tokens_saved = (tokens_saved_per_turn as u64) * monthly_requests;
        let monthly_dollar_savings =
            (monthly_tokens_saved as f64 / 1_000_000.0) * price_per_million_tokens;

        BenchmarkStats {
            total_tools: 0,
            original_bytes: original_tokens * 4,
            compressed_bytes: compressed_tokens * 4,
            original_tokens_est: original_tokens,
            compressed_tokens_est: compressed_tokens,
            tokens_saved_per_turn,
            monthly_tokens_saved,
            monthly_dollar_savings,
        }
    }
}
