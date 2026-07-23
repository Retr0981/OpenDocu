/*
 * opendocu.h — C ABI for the OpenDocu document reduction library.
 *
 * Build artifact: libopendocu.{dylib,so,a} from the `opendocu-ffi` crate.
 *
 * Usage pattern (C):
 *
 *     const char* opts = "{\"level\":\"medium\"}";
 *     void* h = opendocu_reduce(doc_bytes, doc_len, opts, strlen(opts));
 *     if (!h) {
 *         char* err = opendocu_last_error();
 *         fprintf(stderr, "opendocu: %s\n", err);
 *         opendocu_free_string(err);
 *         return 1;
 *     }
 *     size_t len = 0;
 *     const char* json = opendocu_result_json(h, &len);
 *     fwrite(json, 1, len, stdout);
 *     opendocu_free_handle(h);
 *
 * See `example.cpp` for the RAII C++ wrapper.
 */
#ifndef OPENDOCU_H
#define OPENDOCU_H

#include <stddef.h>  /* size_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to a reduction result. */
typedef void opendocu_handle;

/*
 * Returns the OpenDocu library version as a null-terminated string.
 * The returned pointer references static storage and must NOT be freed.
 */
const char* opendocu_version(void);

/*
 * Reduce a document.
 *
 *   input_ptr  : pointer to the raw document bytes (UTF-8 for text formats).
 *   input_len  : number of bytes in the input.
 *   opts_ptr   : optional JSON string with ProcessingOptions, or NULL for
 *                defaults. Example: {"level":"aggressive","output_format":"markdown"}
 *   opts_len   : length of opts_ptr in bytes (0 if opts_ptr is NULL).
 *
 * Returns: a non-null handle on success, or NULL on failure. On failure, call
 * opendocu_last_error() to retrieve a message. The caller owns the handle and
 * must release it with opendocu_free_handle().
 */
opendocu_handle* opendocu_reduce(const unsigned char* input_ptr,
                                 size_t input_len,
                                 const char* opts_ptr,
                                 size_t opts_len);

/*
 * Borrow a handle and obtain the reduction result as a JSON string.
 *
 *   handle : a handle previously returned by opendocu_reduce.
 *   out_len : filled with the length of the returned string in bytes
 *             (may be NULL if you don't need the length).
 *
 * Returns: a pointer to a UTF-8 JSON buffer. The memory is owned by the handle
 * and remains valid until opendocu_free_handle() is called on it. Do NOT free
 * this pointer with opendocu_free_string().
 *
 * The JSON shape matches opendocu_structures::ReducedDocument:
 *   {
 *     "title": string | null,
 *     "source_format": "plaintext" | "markdown" | "html" | "pdf" | "docx" | "epub",
 *     "summary": string,
 *     "key_points": [string, ...],
 *     "keywords": [string, ...],
 *     "metrics": {
 *       "original_words": number, "reduced_words": number,
 *       "retention_ratio": number, "reduction_percent": number,
 *       "original_sentences": number, "key_points": number, "keywords": number,
 *       "processing_seconds": number
 *     }
 *   }
 */
const char* opendocu_result_json(opendocu_handle* handle, size_t* out_len);

/*
 * Returns the last error message for the calling thread, or NULL if there is
 * none. The caller owns the returned string and must free it with
 * opendocu_free_string().
 */
char* opendocu_last_error(void);

/*
 * Free a string previously returned by opendocu_last_error().
 *
 * Do NOT pass pointers from opendocu_version() (static) or
 * opendocu_result_json() (owned by the handle) to this function.
 */
void opendocu_free_string(char* ptr);

/*
 * Free a reduction handle returned by opendocu_reduce(). After this call, any
 * pointer previously obtained from opendocu_result_json() on this handle is
 * invalid.
 */
void opendocu_free_handle(opendocu_handle* handle);

#ifdef __cplusplus
}  /* extern "C" */
#endif

#endif /* OPENDOCU_H */
