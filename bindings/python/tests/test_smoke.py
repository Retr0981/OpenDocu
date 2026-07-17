"""Smoke test: loads the real libopendocu and exercises the Python binding.

Requires `cargo build --release -p opendocu-ffi` to have run first, or
OPENDOCU_LIB_PATH to point at the shared library.
"""

import os
import pytest

import opendocu
from opendocu import OpenDocuError

SAMPLE = """# OpenDocu

OpenDocu is a document reduction platform built in Rust.
It processes documents quickly and supports many formats.
The system handles PDF, DOCX, Markdown, and plain text files.

## Features

The core library provides summarization and key point extraction.
Users can choose light, medium, or aggressive reduction levels.
Streaming output enables real-time processing of large documents.
"""


def test_version_is_semver():
    v = opendocu.version()
    assert isinstance(v, str)
    assert v.count(".") >= 2


def test_reduce_returns_dict():
    result = opendocu.reduce(SAMPLE, level="medium")
    assert isinstance(result, dict)
    assert isinstance(result["summary"], str)
    assert len(result["summary"]) > 0
    assert isinstance(result["key_points"], list)
    assert isinstance(result["keywords"], list)
    assert result["metrics"]["original_words"] > 10
    assert 0 < result["metrics"]["reduced_words"] <= result["metrics"]["original_words"]


def test_reduce_accepts_bytes():
    result = opendocu.reduce(SAMPLE.encode("utf-8"))
    assert result["metrics"]["original_words"] > 0


def test_aggressive_reduces_more():
    light = opendocu.reduce(SAMPLE, level="light")
    aggressive = opendocu.reduce(SAMPLE, level="aggressive")
    assert aggressive["metrics"]["reduction_percent"] >= light["metrics"]["reduction_percent"] - 5


def test_reduce_empty_raises():
    with pytest.raises(OpenDocuError):
        opendocu.reduce("")


def test_reduce_file(tmp_path):
    p = tmp_path / "sample.md"
    p.write_text(SAMPLE)
    result = opendocu.reduce_file(str(p), level="medium")
    assert result["metrics"]["original_words"] > 0


# ---------------------------------------------------------------------------
# Document-to-data extraction
# ---------------------------------------------------------------------------

INVOICE = (
    "Invoice #INV-2024-001\n"
    "Date: 2024-03-15\n"
    "Total: $1,250.00\n"
    "Contact: billing@example.com\n"
    "Visit https://opendocu.org"
)


def test_extract_returns_entities():
    result = opendocu.extract(INVOICE)
    assert isinstance(result, dict)
    types = [e["entity_type"] for e in result["entities"]]
    assert "date" in types
    assert "email" in types
    assert "money" in types
    assert "url" in types


def test_extract_returns_key_values():
    result = opendocu.extract("Name: Alice\nDate: 2024-03-15")
    fields = result["key_values"]["fields"]
    assert len(fields) >= 2
    assert any(f["key"] == "Name" for f in fields)


def test_extract_option_flags():
    result = opendocu.extract(INVOICE, extract_entities=False)
    assert len(result["entities"]) == 0


def test_extract_text_alias():
    result = opendocu.extract_text("Total: $500.00")
    assert any(e["entity_type"] == "money" for e in result["entities"])


def test_extract_empty_raises():
    with pytest.raises(OpenDocuError):
        opendocu.extract("")


if __name__ == "__main__":
    # Allow running without pytest for a quick manual check.
    test_version_is_semver()
    test_reduce_returns_dict()
    test_reduce_accepts_bytes()
    test_aggressive_reduces_more()
    test_reduce_empty_raises()
    print("All manual checks passed.")
