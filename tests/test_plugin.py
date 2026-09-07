"""Unit tests for schemashrink Python bindings and Hermes Agora plugin interface."""

import json
import pytest
from schemashrink import (
    compress,
    compress_schema_py,
    COMPRESS_TOOL_SCHEMA,
    BENCHMARK_TOOL_SCHEMA,
    _handle_compress,
    _handle_benchmark,
    _handle_slash_command,
    register,
)


def test_compress_schema_py_lossless():
    sample_tools = [
        {
            "type": "function",
            "function": {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "execute_code",
                "name": "execute_code",
                "description": "Execute code snippet.",
                "parameters": {
                    "type": "object",
                    "additionalProperties": False,
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Source code.",
                        },
                    },
                    "required": ["code"],
                },
            },
        }
    ]

    res = compress_schema_py(sample_tools)
    assert res["compressed_chars"] < res["original_chars"]
    assert res["token_savings_pct"] > 30.0

    # Verify structural integrity
    comp_data = res["compressed_data"]
    assert comp_data[0]["function"]["name"] == "execute_code"
    assert "$schema" not in comp_data[0]["function"]
    assert "title" not in comp_data[0]["function"]
    assert comp_data[0]["function"]["parameters"]["properties"]["code"]["type"] == "string"
    assert comp_data[0]["function"]["parameters"]["required"] == ["code"]


def test_compress_strip_descriptions():
    sample = {
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search query text",
            }
        },
    }
    res = compress(sample, strip_descriptions=True)
    comp_data = res["compressed_data"]
    assert "description" not in comp_data["properties"]["query"]
    assert comp_data["properties"]["query"]["type"] == "string"


def test_compress_prune_descriptions():
    sample = {
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "A very long detailed string description that exceeds the maximum length limit of thirty chars",
            }
        },
    }
    res = compress(sample, prune_descriptions=True, max_desc_len=20)
    comp_data = res["compressed_data"]
    desc = comp_data["properties"]["query"]["description"]
    assert len(desc) == 20
    assert desc.endswith("...")


def test_tool_handlers():
    sample = json.dumps({
        "type": "object",
        "properties": {
            "test": {"type": "number", "description": "some number"}
        }
    })

    comp_output = _handle_compress({"schema_json": sample})
    assert "⚡ SchemaShrink Summary" in comp_output
    assert "Savings:" in comp_output

    bench_output = _handle_benchmark({"schema_json": sample, "monthly_requests": 500000})
    assert "💰 SchemaShrink Cost Benchmark" in bench_output
    assert "Monthly Requests:" in bench_output


def test_slash_command():
    help_text = _handle_slash_command("help")
    assert "/schemashrink" in help_text

    proxy_text = _handle_slash_command("proxy 20129")
    assert "schemashrink proxy --port 20129" in proxy_text


class MockPluginContext:
    def __init__(self):
        self.tools = []
        self.commands = []

    def register_tool(self, **kwargs):
        self.tools.append(kwargs)

    def register_command(self, **kwargs):
        self.commands.append(kwargs)


def test_hermes_plugin_registration():
    ctx = MockPluginContext()
    register(ctx)

    tool_names = [t["name"] for t in ctx.tools]
    assert "schemashrink_compress" in tool_names
    assert "schemashrink_benchmark" in tool_names
    assert len(ctx.commands) == 1
    assert ctx.commands[0]["name"] == "schemashrink"
