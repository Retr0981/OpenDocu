// Implementation of the OpenDocu C++ wrapper.
//
// We hand-roll a tiny JSON serializer (for options) and parser (for the
// ReducedDocument result) to avoid pulling a JSON dependency into a header
// that is meant to be vendored easily.

#include "opendocu.hpp"

#include <charconv>
#include <sstream>

namespace opendocu {

std::string options::to_json() const {
    std::ostringstream ss;
    ss << '{';
    ss << "\"level\":\"" << level << '"';
    ss << ",\"output_format\":\"" << output_format << '"';
    if (max_sentences > 0) ss << ",\"max_sentences\":" << max_sentences;
    if (max_key_points > 0) ss << ",\"max_key_points\":" << max_key_points;
    if (max_keywords > 0) ss << ",\"max_keywords\":" << max_keywords;
    if (!format_hint.empty()) ss << ",\"format_hint\":\"" << format_hint << '"';
    ss << ",\"abstractive\":" << (abstractive ? "true" : "false");
    if (min_sentence_words != 4) ss << ",\"min_sentence_words\":" << min_sentence_words;
    ss << '}';
    return ss.str();
}

namespace {

/// Skip whitespace in `sv`, advancing `pos`.
void skip_ws(std::string_view sv, std::size_t& pos) {
    while (pos < sv.size() && (sv[pos] == ' ' || sv[pos] == '\n' || sv[pos] == '\r' || sv[pos] == '\t')) {
        ++pos;
    }
}

/// Parse a JSON string literal starting at `pos` (assumed to point at '"').
/// Returns the decoded contents; throws on malformed input.
std::string parse_string(std::string_view sv, std::size_t& pos) {
    if (pos >= sv.size() || sv[pos] != '"') throw error("expected string");
    ++pos;
    std::string out;
    while (pos < sv.size() && sv[pos] != '"') {
        if (sv[pos] == '\\' && pos + 1 < sv.size()) {
            char esc = sv[pos + 1];
            switch (esc) {
                case 'n': out.push_back('\n'); break;
                case 't': out.push_back('\t'); break;
                case 'r': out.push_back('\r'); break;
                case '"': out.push_back('"'); break;
                case '\\': out.push_back('\\'); break;
                case '/': out.push_back('/'); break;
                default: out.push_back(esc); break;
            }
            pos += 2;
        } else {
            out.push_back(sv[pos++]);
        }
    }
    if (pos >= sv.size()) throw error("unterminated string");
    ++pos;  // consume closing quote
    return out;
}

/// Parse a JSON number (int or float) starting at `pos` as a double.
double parse_number(std::string_view sv, std::size_t& pos) {
    std::size_t start = pos;
    if (pos < sv.size() && (sv[pos] == '-' || sv[pos] == '+')) ++pos;
    while (pos < sv.size() &&
           (sv[pos] == '.' || sv[pos] == 'e' || sv[pos] == 'E' ||
            (sv[pos] >= '0' && sv[pos] <= '9') || sv[pos] == '+' || sv[pos] == '-')) {
        ++pos;
    }
    std::string token(sv.substr(start, pos - start));
    try {
        return std::stod(token);
    } catch (...) {
        throw error("bad number: " + token);
    }
}

/// Parse a JSON string array starting at `pos` (assumed to point at '[').
std::vector<std::string> parse_string_array(std::string_view sv, std::size_t& pos) {
    std::vector<std::string> out;
    skip_ws(sv, pos);
    if (pos >= sv.size() || sv[pos] != '[') throw error("expected array");
    ++pos;
    skip_ws(sv, pos);
    if (pos < sv.size() && sv[pos] == ']') { ++pos; return out; }

    while (pos < sv.size()) {
        skip_ws(sv, pos);
        out.push_back(parse_string(sv, pos));
        skip_ws(sv, pos);
        if (pos < sv.size() && sv[pos] == ',') { ++pos; continue; }
        if (pos < sv.size() && sv[pos] == ']') { ++pos; return out; }
        break;
    }
    throw error("malformed array");
}

}  // namespace

reduced_document reducer::parse_json(std::string_view json) {
    reduced_document doc;
    std::size_t pos = 0;
    skip_ws(json, pos);
    if (pos >= json.size() || json[pos] != '{') throw error("expected object");
    ++pos;

    while (pos < json.size()) {
        skip_ws(json, pos);
        if (json[pos] == '}') { ++pos; break; }
        std::string key = parse_string(json, pos);
        skip_ws(json, pos);
        if (pos >= json.size() || json[pos] != ':') throw error("expected ':'");
        ++pos;
        skip_ws(json, pos);

        if (key == "title") {
            if (pos < json.size() && json[pos] == 'n') {
                // "null"
                pos += 4;
            } else {
                doc.title = parse_string(json, pos);
            }
        } else if (key == "source_format") {
            doc.source_format = parse_string(json, pos);
        } else if (key == "summary") {
            doc.summary = parse_string(json, pos);
        } else if (key == "key_points") {
            doc.key_points = parse_string_array(json, pos);
        } else if (key == "keywords") {
            doc.keywords = parse_string_array(json, pos);
        } else if (key == "metrics") {
            // Parse the metrics sub-object.
            if (pos < json.size() && json[pos] == '{') {
                ++pos;
                while (pos < json.size()) {
                    skip_ws(json, pos);
                    if (json[pos] == '}') { ++pos; break; }
                    std::string mkey = parse_string(json, pos);
                    skip_ws(json, pos);
                    if (json[pos] != ':') throw error("expected ':' in metrics");
                    ++pos;
                    skip_ws(json, pos);
                    double val = parse_number(json, pos);
                    if (mkey == "original_words") doc.original_words = static_cast<int>(val);
                    else if (mkey == "reduced_words") doc.reduced_words = static_cast<int>(val);
                    else if (mkey == "retention_ratio") doc.retention_ratio = val;
                    else if (mkey == "reduction_percent") doc.reduction_percent = val;
                    else if (mkey == "original_sentences") doc.original_sentences = static_cast<int>(val);
                    else if (mkey == "processing_seconds") doc.processing_seconds = val;
                    skip_ws(json, pos);
                    if (pos < json.size() && json[pos] == ',') ++pos;
                }
            }
        } else {
            // Skip unknown value: strings, numbers, arrays, booleans.
            if (pos < json.size() && json[pos] == '"') {
                (void)parse_string(json, pos);
            } else if (pos < json.size() && json[pos] == '[') {
                (void)parse_string_array(json, pos);
            } else {
                (void)parse_number(json, pos);
            }
        }

        skip_ws(json, pos);
        if (pos < json.size() && json[pos] == ',') ++pos;
    }

    return doc;
}

}  // namespace opendocu
