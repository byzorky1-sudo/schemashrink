use clap::{Parser, Subcommand};
use schemashrink::{
    benchmark::BenchmarkRunner,
    compressor::{CompressionConfig, SchemaCompressor},
    proxy::start_proxy_server,
    report::{print_benchmark_summary, print_compression_summary},
};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "schemashrink",
    about = "⚡ Zero-latency lossless JSON Schema compressor & prefix KV-cache optimizer for AI Agent Tool Calling.",
    version = "0.2.0",
    author = "byzorky1-sudo (Ali)"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to a JSON schema file to compress
    #[arg(short, long)]
    file: Option<PathBuf>,

    /// Output compressed schema to file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start transparent auto-proxy (Zero-Config for all CLI agents: Hermes, Claude, Codex, Aider)
    Proxy {
        /// Local port for the proxy to listen on
        #[arg(short, long, default_value_t = 20129)]
        port: u16,

        /// Upstream API base URL (e.g. your 9Router or OpenAI endpoint)
        #[arg(short, long, default_value = "http://127.0.0.1:20128")]
        upstream: String,
    },
    /// Compress a JSON schema or tools array file
    Compress {
        /// Path to target schema JSON file
        path: PathBuf,
        /// Output path for compressed JSON
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Prune verbose descriptions
        #[arg(long, default_value_t = false)]
        prune_descriptions: bool,
    },
    /// Model dollar and token savings for production traffic
    Benchmark {
        /// Target JSON schema or tools array file
        path: PathBuf,
        /// Estimated monthly agent turns / requests
        #[arg(long, default_value_t = 1000000)]
        requests: u64,
        /// Input token price per 1M tokens in USD
        #[arg(long, default_value_t = 3.0)]
        price_per_m: f64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Proxy { port, upstream }) => {
            start_proxy_server(port, upstream).await;
        }
        Some(Commands::Compress {
            path,
            output,
            prune_descriptions,
        }) => {
            run_compress(&path, output, prune_descriptions);
        }
        Some(Commands::Benchmark {
            path,
            requests,
            price_per_m,
        }) => {
            run_benchmark(&path, requests, price_per_m);
        }
        None => {
            if let Some(file_path) = cli.file {
                run_compress(&file_path, cli.output, false);
            } else {
                println!("⚡ schemashrink v0.2.0 — Zero-Config Transparent Proxy & Schema Optimizer");
                println!("Run `schemashrink proxy --port 20129 --upstream http://127.0.0.1:20128` to start auto-shrinking!");
            }
        }
    }
}

fn run_compress(path: &PathBuf, output: Option<PathBuf>, prune_descriptions: bool) {
    if !path.exists() {
        eprintln!("❌ Error: File not found at {}", path.display());
        std::process::exit(1);
    }

    let content = fs::read_to_string(path).expect("Failed to read schema file");
    let config = CompressionConfig {
        strip_meta: true,
        deterministic_sort: true,
        prune_descriptions,
        max_desc_len: 120,
        minify: true,
    };

    let compressor = SchemaCompressor::new(config);
    match compressor.compress_str(&content) {
        Ok(res) => {
            print_compression_summary(&res);

            if let Some(out_path) = output {
                fs::write(&out_path, &res.compressed_json)
                    .expect("Failed to write compressed schema");
                println!("💾 Saved compressed schema to: {}\n", out_path.display());
            } else {
                println!("📄 Compressed JSON Output:\n{}\n", res.compressed_json);
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to parse JSON Schema: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_benchmark(path: &PathBuf, requests: u64, price_per_m: f64) {
    if !path.exists() {
        eprintln!("❌ Error: File not found at {}", path.display());
        std::process::exit(1);
    }

    let content = fs::read_to_string(path).expect("Failed to read schema file");
    let compressor = SchemaCompressor::with_default();

    match compressor.compress_str(&content) {
        Ok(res) => {
            let stats = BenchmarkRunner::calculate_savings(
                res.original_tokens_est,
                res.compressed_tokens_est,
                requests,
                price_per_m,
            );
            print_benchmark_summary(&stats, requests, price_per_m);
        }
        Err(e) => {
            eprintln!("❌ Error parsing JSON: {}", e);
        }
    }
}
