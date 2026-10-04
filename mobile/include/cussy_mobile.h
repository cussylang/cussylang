#ifndef CUSSY_MOBILE_H
#define CUSSY_MOBILE_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define CUSSY_MOBILE_MAX_SOURCE_BYTES ((size_t)1048576)
#define CUSSY_MOBILE_DEFAULT_FUEL UINT64_C(5000000)
#define CUSSY_MOBILE_MAX_FUEL UINT64_C(5000000)

/* ABI contract version, independent of the language/package version. */
uint32_t cussy_mobile_api_version(void);

/*
 * Run UTF-8 Cussy source synchronously using an internal 16 MiB worker stack.
 * Call this function from an app background task, not the UI thread.
 *
 * source must point to source_len readable bytes for the duration of this call.
 * NULL is allowed only when source_len is zero. Source is limited to 1 MiB.
 * A zero fuel value selects the default; larger values are capped at MAX_FUEL.
 * Source also has limits of 8192 tokens total and 1024 tokens between statement
 * or block boundaries. Comments and string contents do not add tokens.
 *
 * Returns an owned, NUL-terminated UTF-8 JSON object with these fields:
 *   ok: boolean -- compilation and execution completed without a diagnostic;
 *   exit_code: signed 64-bit integer on success, otherwise null;
 *   output: captured UTF-8 output, including output preceding a runtime error;
 *   diagnostic: readable diagnostic text, or an empty string on success.
 * A nonzero Cussy exit code is still a completed run with ok=true.
 * The caller must handle NULL if no response could be produced. Process-aborting
 * failures, including out-of-memory and stack overflow, are not recovered.
 *
 * Only embedded standard imports are available. Host filesystem, graph-file
 * output, native FFI, sleeping, stdin and environment access are disabled.
 * This is an in-process interpreter, not a memory/CPU isolation boundary.
 * No process-wide cwd, environment, or output configuration is changed.
 *
 * Free a non-NULL result exactly once using cussy_mobile_free from this library.
 */
char *cussy_mobile_run(const uint8_t *source, size_t source_len, uint64_t fuel);

/* NULL is a no-op. Other pointers must be live results from cussy_mobile_run. */
void cussy_mobile_free(char *result);

#ifdef __cplusplus
}
#endif
#endif
