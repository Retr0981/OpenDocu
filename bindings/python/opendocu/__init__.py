"""Python bindings for OpenDocu.

Loads the native ``libopendocu`` library produced by the Rust ``ffi`` crate
and exposes a small, Pythonic API over the C ABI.

Example::

    import opendocu
    result = opendocu.reduce("# Title\\n\\nLong body text...", level="medium")
    print(result["summary"])

The native library is located via, in order:
  1. the ``OPENDOCU_LIB_PATH`` environment variable,
  2. ``../../target/release`` relative to this package (a local build),
  3. the system loader's default search paths.
"""

from __future__ import annotations

import ctypes
import json
import os
import pathlib
import sys
from typing import Any, Dict, Optional, Union

__version__ = "0.1.0"

__all__ = ["reduce", "reduce_file", "extract", "extract_text", "version", "OpenDocuError", "__version__"]


class OpenDocuError(Exception):
    """Raised when the native library reports a failure."""


def _lib_basename() -> str:
    if sys.platform == "darwin":
        return "libopendocu.dylib"
    if sys.platform.startswith("linux"):
        return "libopendocu.so"
    if sys.platform == "win32":
        return "opendocu.dll"
    return "libopendocu.so"


def _resolve_lib_path() -> str:
    env = os.environ.get("OPENDOCU_LIB_PATH")
    if env:
        return env

    here = pathlib.Path(__file__).resolve().parent
    candidates = [
        here.parent.parent.parent / "target" / "release" / _lib_basename(),
        here.parent.parent.parent / "target" / "debug" / _lib_basename(),
    ]
    for c in candidates:
        if c.exists():
            return str(c)

    # Fall back to the bare name so the system loader (DYLD/LD path) can find it.
    return _lib_basename()


_lib = None


def _load():
    global _lib
    if _lib is not None:
        return _lib

    path = _resolve_lib_path()
    try:
        lib = ctypes.CDLL(path)
    except OSError as exc:
        raise OpenDocuError(
            f"Could not load OpenDocu native library at {path!r}: {exc}. "
            "Build it with `cargo build --release -p opendocu-ffi` or set "
            "OPENDOCU_LIB_PATH."
        ) from exc

    # Declare the C function signatures.
    lib.opendocu_version.restype = ctypes.c_char_p
    lib.opendocu_version.argtypes = []

    lib.opendocu_reduce.restype = ctypes.c_void_p
    lib.opendocu_reduce.argtypes = [
        ctypes.c_char_p,        # input_ptr
        ctypes.c_size_t,        # input_len
        ctypes.c_char_p,        # opts_ptr
        ctypes.c_size_t,        # opts_len
    ]

    lib.opendocu_extract.restype = ctypes.c_void_p
    lib.opendocu_extract.argtypes = [
        ctypes.c_char_p,        # input_ptr
        ctypes.c_size_t,        # input_len
        ctypes.c_char_p,        # opts_ptr
        ctypes.c_size_t,        # opts_len
    ]

    lib.opendocu_result_json.restype = ctypes.c_char_p
    lib.opendocu_result_json.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_size_t)]

    lib.opendocu_last_error.restype = ctypes.POINTER(ctypes.c_char)
    lib.opendocu_last_error.argtypes = []

    lib.opendocu_free_string.restype = None
    lib.opendocu_free_string.argtypes = [ctypes.POINTER(ctypes.c_char)]

    lib.opendocu_free_handle.restype = None
    lib.opendocu_free_handle.argtypes = [ctypes.c_void_p]

    _lib = lib
    return lib


def _last_error(lib) -> Optional[str]:
    ptr = lib.opendocu_last_error()
    if not ptr:
        return None
    try:
        return ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8", "replace")
    finally:
        lib.opendocu_free_string(ptr)


def version() -> str:
    """Return the OpenDocu native library version."""
    lib = _load()
    raw = lib.opendocu_version()
    return raw.decode("utf-8") if raw else ""


def reduce(
    data: Union[bytes, str],
    level: str = "medium",
    output_format: str = "markdown",
    **kwargs: Any,
) -> Dict[str, Any]:
    """Reduce a document and return the result as a dict.

    Parameters
    ----------
    data : bytes or str
        The document content. ``str`` is encoded as UTF-8.
    level : str
        One of ``"light"``, ``"medium"``, ``"aggressive"``.
    output_format : str
        One of ``"plaintext"``, ``"markdown"``, ``"json"``.
    **kwargs
        Additional :class:`ProcessingOptions` fields forwarded to Rust:
        ``max_sentences``, ``max_key_points``, ``max_keywords``,
        ``format_hint``, ``abstractive``, ``min_sentence_words``.
    """
    lib = _load()

    if isinstance(data, str):
        data = data.encode("utf-8")
    if not data:
        raise OpenDocuError("input is empty")

    opts = {"level": level, "output_format": output_format, **kwargs}
    opts_json = json.dumps(opts).encode("utf-8")

    handle = lib.opendocu_reduce(data, len(data), opts_json, len(opts_json))
    if not handle:
        msg = _last_error(lib) or "opendocu_reduce failed"
        raise OpenDocuError(msg)

    try:
        length = ctypes.c_size_t(0)
        raw = lib.opendocu_result_json(handle, ctypes.byref(length))
        if not raw:
            msg = _last_error(lib) or "opendocu_result_json failed"
            raise OpenDocuError(msg)
        return json.loads(raw[: length.value].decode("utf-8"))
    finally:
        lib.opendocu_free_handle(handle)


def reduce_file(path: Union[str, os.PathLike], **kwargs: Any) -> Dict[str, Any]:
    """Read a file from disk and reduce it. Accepts the same kwargs as :func:`reduce`."""
    with open(path, "rb") as fh:
        return reduce(fh.read(), **kwargs)


# ---------------------------------------------------------------------------
# Document-to-data extraction
# ---------------------------------------------------------------------------

def extract(
    data: Union[bytes, str],
    extract_tables: bool = True,
    extract_key_values: bool = True,
    extract_entities: bool = True,
) -> Dict[str, Any]:
    """Extract structured data (tables, key-value pairs, typed entities) from
    a document.

    Turns unstructured document content into LLM-ready JSON with tables,
    key-value fields, and typed entities (dates, emails, money, identifiers).

    Parameters
    ----------
    data : bytes or str
        The document content. ``str`` is encoded as UTF-8.
    extract_tables : bool
        Detect tables from whitespace/pipe/tab alignment.
    extract_key_values : bool
        Detect form-style "Key: Value" pairs.
    extract_entities : bool
        Mine typed entities (dates, emails, money, etc.).

    Returns
    -------
    dict
        Keys: ``tables``, ``key_values``, ``entities``, ``field_count``,
        ``confidence``, ``provider``.

    Example
    -------
    >>> result = opendocu.extract("Invoice #123\\nDate: 2024-03-15\\nTotal: $1,250.00")
    >>> [e["entity_type"] for e in result["entities"]]
    ['identifier', 'date', 'money']
    """
    lib = _load()

    if isinstance(data, str):
        data = data.encode("utf-8")
    if not data:
        raise OpenDocuError("input is empty")

    opts = json.dumps({
        "extract_tables": extract_tables,
        "extract_key_values": extract_key_values,
        "extract_entities": extract_entities,
    }).encode("utf-8")

    handle = lib.opendocu_extract(data, len(data), opts, len(opts))
    if not handle:
        msg = _last_error(lib) or "opendocu_extract failed"
        raise OpenDocuError(msg)

    try:
        length = ctypes.c_size_t(0)
        raw = lib.opendocu_result_json(handle, ctypes.byref(length))
        if not raw:
            msg = _last_error(lib) or "opendocu_result_json failed"
            raise OpenDocuError(msg)
        return json.loads(raw[: length.value].decode("utf-8"))
    finally:
        lib.opendocu_free_handle(handle)


def extract_text(
    text: Union[bytes, str],
    **kwargs: Any,
) -> Dict[str, Any]:
    """Extract structured data from a plain-text string. Alias for :func:`extract`
    that emphasizes text-only input."""
    return extract(text, **kwargs)
