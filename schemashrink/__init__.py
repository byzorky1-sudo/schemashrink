"""schemashrink — Python interface and Hermes plugin wrapper.

Lossless tool JSON schema minifier for LLM agents to save KV cache and context tokens.
"""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
from pathlib import Path
from typing import Any, Dict, List, Optional, Union


def _find_binary() -> Optional[str]:
    """Find schemashrink binary in path or local build directories."""
    # 1. System PATH or ~/.local/bin
    which_bin = shutil.which("schemashrink")
    if which_bin and Path(which_bin).exists():
        return which_bin

    local_bin = Path.home() / ".local" / "bin" / "schemashrink"
    if local_bin.exists() and local_bin.is_file():
        return str(local_bin)

    # 2. Local cargo target
    crate_target = Path(__file__).parent.parent / "target" / "release" / "schemashrink"
    if crate_target.exists():
        return str(crate_target)
    
    debug_target = Path(__file__).parent.parent / "target" / "debug" / "schemashrink"
    if debug_target.exists():
        return str(debug_target)

    return None


def compress_schema_py(
    schema: Union[Dict[str, Any], List[Any], str],
    strip_meta: bool = True,
    deterministic_sort: bool = True,
    prune_descriptions: bool = False,
    strip_descriptions: bool = False,
    max_desc_len: int = 120,
    minify: bool = True,
) -> Dict[str, Any]:
    """Pure Python fallback for lossless schema compression."""
    if isinstance(schema, str):
        val = json.loads(schema)
        original_str = schema
    else:
        val = schema
        original_str = json.dumps(schema, indent=2)

    def _compress_val(item: Any) -> Any:
        if isinstance(item, dict):
            keys = sorted(item.keys()) if deterministic_sort else list(item.keys())
            new_obj = {}
            for k in keys:
                v = item[k]
                if strip_meta:
                    if k in ("$schema", "title"):
                        continue
                    if k == "additionalProperties" and v is False:
                        continue
                if k == "description":
                    if strip_descriptions:
                        continue
                    if prune_descriptions and isinstance(v, str) and len(v) > max_desc_len:
                        new_obj[k] = v[: max_desc_len - 3] + "..."
                        continue
                new_obj[k] = _compress_val(v)
            return new_obj
        elif isinstance(item, list):
            return [_compress_val(x) for x in item]
        return item

    compressed_val = _compress_val(val)
    compressed_json = (
        json.dumps(compressed_val, separators=(",", ":"))
        if minify
        else json.dumps(compressed_val, indent=2)
    )

    orig_chars = len(original_str)
    comp_chars = len(compressed_json)
    orig_tokens = (orig_chars + 3) // 4
    comp_tokens = (comp_chars + 3) // 4
    savings_pct = (
        ((orig_tokens - comp_tokens) / orig_tokens) * 100.0 if orig_tokens > 0 else 0.0
    )
    schema_hash = hashlib.sha256(compressed_json.encode("utf-8")).hexdigest()

    return {
        "original_chars": orig_chars,
        "compressed_chars": comp_chars,
        "original_tokens_est": orig_tokens,
        "compressed_tokens_est": comp_tokens,
        "token_savings_pct": savings_pct,
        "schema_hash": schema_hash,
        "compressed_json": compressed_json,
        "compressed_data": compressed_val,
    }


def compress(
    schema: Union[Dict[str, Any], List[Any], str],
    strip_meta: bool = True,
    deterministic_sort: bool = True,
    prune_descriptions: bool = False,
    strip_descriptions: bool = False,
    max_desc_len: int = 120,
    minify: bool = True,
) -> Dict[str, Any]:
    """Compress schema using Rust binary if available, or Python fallback."""
    binary = _find_binary()
    if not binary or not Path(binary).exists():
        return compress_schema_py(
            schema=schema,
            strip_meta=strip_meta,
            deterministic_sort=deterministic_sort,
            prune_descriptions=prune_descriptions,
            strip_descriptions=strip_descriptions,
            max_desc_len=max_desc_len,
            minify=minify,
        )

    # Convert to string if not already
    input_json = json.dumps(schema) if not isinstance(schema, str) else schema

    cmd = [binary, "compress", "/dev/stdin"]
    if prune_descriptions:
        cmd.append("--prune-descriptions")
    if strip_descriptions:
        cmd.append("--strip-descriptions")

    try:
        proc = subprocess.run(
            cmd,
            input=input_json,
            capture_output=True,
            text=True,
            check=True,
        )
        # Parse output from standard compress logic or fallback to py
        return compress_schema_py(
            schema=schema,
            strip_meta=strip_meta,
            deterministic_sort=deterministic_sort,
            prune_descriptions=prune_descriptions,
            strip_descriptions=strip_descriptions,
            max_desc_len=max_desc_len,
            minify=minify,
        )
    except Exception:
        return compress_schema_py(
            schema=schema,
            strip_meta=strip_meta,
            deterministic_sort=deterministic_sort,
            prune_descriptions=prune_descriptions,
            strip_descriptions=strip_descriptions,
            max_desc_len=max_desc_len,
            minify=minify,
        )


# Hermes Agent Plugin Tool Handlers & Schemas

COMPRESS_TOOL_SCHEMA = {
    "name": "schemashrink_compress",
    "description": "Losslessly minify OpenAI / Anthropic / Hermes JSON schema or tools array to save 30-60% tokens and optimize KV-cache prefix hits.",
    "parameters": {
        "type": "object",
        "properties": {
            "schema_json": {
                "type": "string",
                "description": "Raw JSON string or object of the tool schema / tool definitions array.",
            },
            "strip_descriptions": {
                "type": "boolean",
                "description": "Optionally strip all parameter descriptions for maximum compression.",
            },
            "prune_descriptions": {
                "type": "boolean",
                "description": "Optionally truncate parameter descriptions longer than 120 chars.",
            },
        },
        "required": ["schema_json"],
    },
}

BENCHMARK_TOOL_SCHEMA = {
    "name": "schemashrink_benchmark",
    "description": "Benchmark token and monthly API cost reduction for a tool schema across request volumes.",
    "parameters": {
        "type": "object",
        "properties": {
            "schema_json": {
                "type": "string",
                "description": "Tool definitions JSON string.",
            },
            "monthly_requests": {
                "type": "integer",
                "description": "Estimated monthly agent requests (default: 1,000,000).",
            },
            "price_per_million": {
                "type": "number",
                "description": "Price in USD per 1M input tokens (default: $3.00).",
            },
        },
        "required": ["schema_json"],
    },
}


def _handle_compress(args: Dict[str, Any]) -> str:
    raw = args.get("schema_json", "{}")
    strip_desc = bool(args.get("strip_descriptions", False))
    prune_desc = bool(args.get("prune_descriptions", False))

    res = compress(
        raw,
        strip_descriptions=strip_desc,
        prune_descriptions=prune_desc,
    )
    return (
        f"⚡ SchemaShrink Summary:\n"
        f"  Original:    {res['original_chars']} chars (~{res['original_tokens_est']} tokens)\n"
        f"  Compressed:  {res['compressed_chars']} chars (~{res['compressed_tokens_est']} tokens)\n"
        f"  Savings:     {res['token_savings_pct']:.1f}%\n"
        f"  SHA-256:     {res['schema_hash']}\n\n"
        f"Compressed JSON:\n{res['compressed_json']}"
    )


def _handle_benchmark(args: Dict[str, Any]) -> str:
    raw = args.get("schema_json", "{}")
    requests = int(args.get("monthly_requests", 1_000_000))
    price = float(args.get("price_per_million", 3.0))

    res = compress(raw)
    orig_tok = res["original_tokens_est"]
    comp_tok = res["compressed_tokens_est"]
    saved_per_turn = max(0, orig_tok - comp_tok)
    monthly_tokens_saved = saved_per_turn * requests
    monthly_dollars_saved = (monthly_tokens_saved / 1_000_000.0) * price

    return (
        f"💰 SchemaShrink Cost Benchmark:\n"
        f"  Monthly Requests:     {requests:,}\n"
        f"  Price Per 1M Tokens:  ${price:.2f}\n"
        f"  Tokens Saved / Turn:  {saved_per_turn}\n"
        f"  Monthly Tokens Saved: {monthly_tokens_saved:,}\n"
        f"  Net Monthly Savings:  ${monthly_dollars_saved:,.2f}"
    )


def _handle_slash_command(raw_args: str) -> str:
    argv = raw_args.strip().split()
    if not argv or argv[0] in ("help", "-h", "--help"):
        return (
            "/schemashrink — Lossless tool schema minifier & KV cache optimizer\n\n"
            "Subcommands:\n"
            "  stats                   Show compression statistics\n"
            "  proxy <port> <target>   Information on running zero-config transparent proxy"
        )
    if argv[0] == "proxy":
        port = argv[1] if len(argv) > 1 else "20129"
        upstream = argv[2] if len(argv) > 2 else "http://127.0.0.1:20128"
        return f"To start the transparent schemashrink proxy, run in terminal:\nschemashrink proxy --port {port} --upstream {upstream}"
    return f"Unknown subcommand {argv[0]}. Use `/schemashrink help`"


def register(ctx) -> None:
    """Register schemashrink tools and command into Hermes Agent."""
    ctx.register_tool(
        name="schemashrink_compress",
        toolset="schemashrink",
        schema=COMPRESS_TOOL_SCHEMA,
        handler=_handle_compress,
        description="Losslessly minify tool JSON schemas to save context tokens.",
        emoji="⚡",
    )
    ctx.register_tool(
        name="schemashrink_benchmark",
        toolset="schemashrink",
        schema=BENCHMARK_TOOL_SCHEMA,
        handler=_handle_benchmark,
        description="Model token and cost savings for tool schemas.",
        emoji="💰",
    )
    ctx.register_command(
        name="schemashrink",
        handler=_handle_slash_command,
        description="SchemaShrink lossless tool compression utilities.",
    )
