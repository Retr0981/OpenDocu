// example.cpp — minimal demonstration of the OpenDocu C++ wrapper.
//
// Build (from the repo root):
//   clang++ -std=c++17 -I crates/ffi -I bindings/cpp \
//     bindings/cpp/example.cpp bindings/cpp/opendocu.cpp \
//     -L target/release -lopendocu -o target/example
//
// Run:
//   DYLD_LIBRARY_PATH=target/release ./target/example

#include "opendocu.hpp"

#include <iostream>

int main() {
    try {
        std::cout << "OpenDocu version: " << opendocu::reducer::version() << '\n';

        std::string sample =
            "# OpenDocu\n\n"
            "OpenDocu is a document reduction platform built in Rust. "
            "It processes documents quickly and supports many formats. "
            "The system handles PDF, DOCX, Markdown, and plain text files.\n\n"
            "## Features\n\n"
            "The core library provides summarization and key point extraction. "
            "Users can choose light, medium, or aggressive reduction levels. "
            "Streaming output enables real-time processing of large documents.\n";

        opendocu::reducer r;
        opendocu::options opts;
        opts.level = "medium";

        auto result = r.reduce(sample, opts);

        std::cout << "\n--- Summary ---\n" << result.summary << '\n';
        std::cout << "\n--- Key points (" << result.key_points.size() << ") ---\n";
        for (const auto& kp : result.key_points) std::cout << "  - " << kp << '\n';
        std::cout << "\n--- Keywords (" << result.keywords.size() << ") ---\n";
        for (const auto& kw : result.keywords) std::cout << "  - " << kw << '\n';

        std::cout << "\n--- Metrics ---\n"
                  << "  original_words:   " << result.original_words << '\n'
                  << "  reduced_words:    " << result.reduced_words << '\n'
                  << "  reduction_percent:" << result.reduction_percent << '\n'
                  << "  processing_secs:  " << result.processing_seconds << '\n';

        // Sanity assertions — exit non-zero if the binding misbehaves.
        if (result.reduced_words == 0) {
            std::cerr << "FAIL: reduced_words is zero\n";
            return 1;
        }
        if (result.summary.empty()) {
            std::cerr << "FAIL: summary is empty\n";
            return 1;
        }
        std::cout << "\nOK: C++ example completed successfully.\n";
        return 0;
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << '\n';
        return 1;
    }
}
