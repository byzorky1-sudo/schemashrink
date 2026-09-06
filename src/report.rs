use crate::benchmark::BenchmarkStats;
use crate::compressor::CompressionResult;

pub fn print_compression_summary(res: &CompressionResult) {
    println!("\n⚡ schemashrink — Zero-Loss JSON Schema Compression Summary:");
    println!("============================================================");
    println!(
        "📏 Original Size:    {:>8} chars (~{} tokens)",
        res.original_chars, res.original_tokens_est
    );
    println!(
        "📦 Compressed Size:  {:>8} chars (~{} tokens)",
        res.compressed_chars, res.compressed_tokens_est
    );
    println!("🚀 Token Reduction:   {:>7.1}%", res.token_savings_pct);
    println!("🔒 Schema SHA-256:   {}", res.schema_hash);
    println!("============================================================\n");
}

pub fn print_benchmark_summary(stats: &BenchmarkStats, requests: u64, price_per_m: f64) {
    println!("\n💰 schemashrink — Production GPU & API Cost Savings Modeler:");
    println!("============================================================");
    println!("📈 Monthly Requests:   {}", requests);
    println!("💵 API Input Price:    ${:.2} / 1M tokens", price_per_m);
    println!(
        "📉 Saved Per Turn:     ~{} tokens",
        stats.tokens_saved_per_turn
    );
    println!(
        "🔥 Monthly Toks Saved: {} tokens",
        stats.monthly_tokens_saved
    );
    println!(
        "🎯 NET MONTHLY SAVING: ${:.2} / month",
        stats.monthly_dollar_savings
    );
    println!("============================================================\n");
}
