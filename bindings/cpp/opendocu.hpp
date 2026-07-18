// opendocu.hpp — RAII C++17 wrapper over the OpenDocu C ABI.
//
// Provides exception-safe ownership of reduction handles and a small value
// type for the JSON result. The underlying C declarations live in opendocu.h.
//
// Example:
//   opendocu::reducer r;
//   auto result = r.reduce("# Title\n\nLong body text...");
//   std::cout << result.summary << '\n';

#ifndef OPENDOCU_HPP
#define OPENDOCU_HPP

#include <cstddef>
#include <cstdint>
#include <memory>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

extern "C" {
#include "opendocu.h"
}

namespace opendocu {

/// Thrown when the native library reports a failure.
class error : public std::runtime_error {
public:
    explicit error(const std::string& msg) : std::runtime_error(msg) {}
};

/// JSON-serializable reduction options. Fields default to match the Rust
/// ProcessingOptions defaults.
struct options {
    std::string level = "medium";             ///< "light" | "medium" | "aggressive"
    std::string output_format = "markdown";   ///< "plaintext" | "markdown" | "json"
    int max_sentences = 0;                    ///< 0 = use level default
    int max_key_points = 0;
    int max_keywords = 0;
    std::string format_hint;                  ///< empty = auto-detect
    bool abstractive = false;
    int min_sentence_words = 4;

    /// Serialize to the JSON shape the Rust ABI expects.
    std::string to_json() const;
};

/// A completed reduction result. Fields mirror ReducedDocument on the Rust side.
struct reduced_document {
    std::string title;          ///< empty if none
    std::string source_format;
    std::string summary;
    std::vector<std::string> key_points;
    std::vector<std::string> keywords;

    int original_words = 0;
    int reduced_words = 0;
    double retention_ratio = 0.0;
    double reduction_percent = 0.0;
    int original_sentences = 0;
    double processing_seconds = 0.0;
};

namespace detail {

/// Custom deleter for the opaque C handle.
struct handle_deleter {
    void operator()(opendocu_handle* h) const noexcept {
        if (h) opendocu_free_handle(h);
    }
};

using handle_ptr = std::unique_ptr<opendocu_handle, handle_deleter>;

/// RAII-allocate a string returned by opendocu_last_error().
struct error_string_deleter {
    void operator()(char* s) const noexcept {
        if (s) opendocu_free_string(s);
    }
};
using error_string_ptr = std::unique_ptr<char, error_string_deleter>;

}  // namespace detail

/// The main reducer type. One instance per thread is sufficient; the underlying
/// library is thread-safe across separate handles.
class reducer {
public:
    reducer() = default;

    /// @returns the library version (static storage, never freed).
    static std::string version() {
        const char* v = opendocu_version();
        return v ? std::string(v) : std::string();
    }

    /// Reduce document bytes.
    reduced_document reduce(const std::vector<std::uint8_t>& input,
                            const options& opts = options{}) const {
        return reduce(input.data(), input.size(), opts);
    }

    /// Reduce a string (UTF-8).
    reduced_document reduce(std::string_view input,
                            const options& opts = options{}) const {
        return reduce(reinterpret_cast<const std::uint8_t*>(input.data()),
                      input.size(), opts);
    }

    /// Reduce from a pointer + length.
    reduced_document reduce(const std::uint8_t* data, std::size_t len,
                            const options& opts) const {
        if (len == 0) throw error("input is empty");
        const std::string opts_json = opts.to_json();

        opendocu_handle* raw = opendocu_reduce(
            data, len, opts_json.data(), opts_json.size());
        if (!raw) {
            throw error(last_error_or("opendocu_reduce failed"));
        }
        detail::handle_ptr handle(raw);

        std::size_t json_len = 0;
        const char* json_ptr = opendocu_result_json(handle.get(), &json_len);
        if (!json_ptr) {
            throw error(last_error_or("opendocu_result_json failed"));
        }
        return parse_json(std::string_view(json_ptr, json_len));
    }

private:
    /// Read and free the thread-local last error.
    static std::string last_error_or(const char* fallback) {
        char* raw = opendocu_last_error();
        if (!raw) return std::string(fallback);
        detail::error_string_ptr guard(raw);
        return std::string(raw);
    }

    /// Minimal JSON parser for the ReducedDocument shape. Avoids a third-party
    /// JSON dependency by hand-reading the fields we control.
    static reduced_document parse_json(std::string_view json);
};

}  // namespace opendocu

#endif  // OPENDOCU_HPP
