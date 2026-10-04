/* Small checked scalar runtime for generated Cussy C. No AST or interpreter. */
#include <stdbool.h>
#include <stdint.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <limits.h>
#ifdef _WIN32
#include <io.h>
#include <fcntl.h>
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <shellapi.h>
#endif

typedef struct { const char *file; size_t line, column; } cx_loc;
typedef struct { const char *data; size_t length; } cx_str;
static unsigned cx_depth;
static size_t cx_output_size;
static int cx_arg_count;
static char **cx_args;
#define CX_LIMIT ((size_t)8 * 1024 * 1024)

static void cx_setup_output(void) {
#ifdef _WIN32
    _setmode(_fileno(stdout), _O_BINARY);
#endif
}

static _Noreturn void cx_fail(const char *code, const char *message, cx_loc location) {
    fprintf(stderr, "error[%s]: %s\n --> %s:%zu:%zu\n", code, message,
            location.file, location.line, location.column);
    exit(1);
}
#ifdef _WIN32
static char **cx_owned_args;
static void cx_cleanup_args(void) {
    if (!cx_owned_args) return;
    for (int i = 0; i < cx_arg_count; ++i) free(cx_owned_args[i]);
    free(cx_owned_args);
    cx_owned_args = NULL;
}
#endif
static void cx_setup_args(int argc, char **argv, cx_loc location) {
    cx_arg_count = argc - 1;
    cx_args = argv + 1;
#ifdef _WIN32
    /* The narrow Windows CRT argv can use an ANSI code page. Decode the original
       command line into Unicode first so Cussy strings always receive UTF-8. */
    int wide_count = 0;
    LPWSTR *wide = CommandLineToArgvW(GetCommandLineW(), &wide_count);
    if (!wide || wide_count < 1) cx_fail("IO", "cannot decode native process arguments", location);
    cx_arg_count = wide_count - 1;
    cx_owned_args = (char **)calloc(cx_arg_count ? (size_t)cx_arg_count : 1, sizeof(char *));
    if (!cx_owned_args) {
        LocalFree(wide);
        cx_fail("LIMIT", "cannot allocate native process arguments", location);
    }
    for (int i = 0; i < cx_arg_count; ++i) {
        int size = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, wide[i + 1], -1, NULL, 0, NULL, NULL);
        if (!size) {
            LocalFree(wide);
            cx_cleanup_args();
            cx_fail("IO", "process argument is not valid Unicode", location);
        }
        cx_owned_args[i] = (char *)malloc((size_t)size);
        if (!cx_owned_args[i]) {
            LocalFree(wide);
            cx_cleanup_args();
            cx_fail("LIMIT", "cannot allocate native process argument", location);
        }
        if (!WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, wide[i + 1], -1, cx_owned_args[i], size, NULL, NULL)) {
            LocalFree(wide);
            cx_cleanup_args();
            cx_fail("IO", "cannot encode native process argument as UTF-8", location);
        }
    }
    LocalFree(wide);
    cx_args = cx_owned_args;
    if (atexit(cx_cleanup_args)) {
        cx_cleanup_args();
        cx_fail("LIMIT", "cannot register native argument cleanup", location);
    }
#else
    (void)location;
#endif
}
static void cx_ready(bool ready, cx_loc location) {
    if (!ready) cx_fail("D404", "global has not been initialized", location);
}
static void cx_check_depth(cx_loc location) {
    if (cx_depth >= 128) cx_fail("LIMIT", "call depth exceeds 128", location);
}
static double cx_finite(double value, cx_loc location) {
    if (!isfinite(value)) cx_fail("DOMAIN", "math domain error or non-finite result", location);
    return value;
}
/* Match Cussy's explicit signed-zero tie rule rather than platform fmin/fmax. */
static double cx_min(double a, double b) {
    if (a == 0.0 && b == 0.0) return signbit(a) || signbit(b) ? -0.0 : 0.0;
    return fmin(a, b);
}
static double cx_max(double a, double b) {
    if (a == 0.0 && b == 0.0) return signbit(a) && signbit(b) ? -0.0 : 0.0;
    return fmax(a, b);
}
static uint64_t cx_unsigned(int64_t value, cx_loc location) {
    if (value < 0) cx_fail("AURA_OVERFLOW", "negative operand in unsigned arithmetic", location);
    return (uint64_t)value;
}
#define CX_CHECKED_ARITH(NAME, TYPE, OP) \
static TYPE NAME(TYPE a, TYPE b, cx_loc location) { \
    TYPE result; \
    if (__builtin_##OP##_overflow(a, b, &result)) \
        cx_fail("AURA_OVERFLOW", "integer overflow or division by zero", location); \
    return result; \
}
CX_CHECKED_ARITH(cx_iadd, int64_t, add)
CX_CHECKED_ARITH(cx_isub, int64_t, sub)
CX_CHECKED_ARITH(cx_imul, int64_t, mul)
CX_CHECKED_ARITH(cx_uadd, uint64_t, add)
CX_CHECKED_ARITH(cx_usub, uint64_t, sub)
CX_CHECKED_ARITH(cx_umul, uint64_t, mul)
#undef CX_CHECKED_ARITH
static int64_t cx_idiv(int64_t a, int64_t b, cx_loc location) {
    if (!b || (a == INT64_MIN && b == -1)) cx_fail("AURA_OVERFLOW", "integer overflow or division by zero", location);
    return a / b;
}
static int64_t cx_irem(int64_t a, int64_t b, cx_loc location) {
    if (!b || (a == INT64_MIN && b == -1)) cx_fail("AURA_OVERFLOW", "integer overflow or division by zero", location);
    return a % b;
}
static uint64_t cx_udiv(uint64_t a, uint64_t b, cx_loc location) {
    if (!b) cx_fail("AURA_OVERFLOW", "integer overflow or division by zero", location);
    return a / b;
}
static uint64_t cx_urem(uint64_t a, uint64_t b, cx_loc location) {
    if (!b) cx_fail("AURA_OVERFLOW", "integer overflow or division by zero", location);
    return a % b;
}
static int64_t cx_ineg(int64_t a, cx_loc location) {
    if (a == INT64_MIN) cx_fail("AURA_OVERFLOW", "integer negation overflow", location);
    return -a;
}
static uint64_t cx_uneg(uint64_t a, cx_loc location) {
    if (a) cx_fail("AURA_OVERFLOW", "cannot negate unsigned value", location);
    return 0;
}
static double cx_fdiv(double a, double b, cx_loc location) {
    if (b == 0) cx_fail("DOMAIN", "invalid operation or division by zero", location);
    return cx_finite(a / b, location);
}
static double cx_frem(double a, double b, cx_loc location) {
    if (b == 0) cx_fail("DOMAIN", "invalid operation or division by zero", location);
    return cx_finite(fmod(a, b), location);
}
static bool cx_string_equal(cx_str a, cx_str b) {
    return a.length == b.length && !memcmp(a.data, b.data, a.length);
}
static size_t cx_chars(cx_str text) {
    size_t count = 0;
    for (size_t i = 0; i < text.length; ++i)
        if (((unsigned char)text.data[i] & 0xc0) != 0x80) ++count;
    return count;
}
static uint32_t cx_string_index(cx_str text, int64_t index, cx_loc location) {
    if (index < 0) cx_fail("DOMAIN", "negative index", location);
    size_t offset = 0;
    for (int64_t n = 0; n < index && offset < text.length; ++n) {
        ++offset;
        while (offset < text.length && ((unsigned char)text.data[offset] & 0xc0) == 0x80) ++offset;
    }
    if (offset == text.length) cx_fail("DOMAIN", "string index out of bounds", location);
    uint32_t result = (unsigned char)text.data[offset++];
    if (result < 0x80) return result;
    result &= result < 0xe0 ? 0x1f : result < 0xf0 ? 0x0f : 0x07;
    while (offset < text.length && ((unsigned char)text.data[offset] & 0xc0) == 0x80)
        result = (result << 6) | ((unsigned char)text.data[offset++] & 0x3f);
    return result;
}
static int64_t cx_parse_int(cx_str text, cx_loc location) {
    size_t i = 0;
    bool negative = false;
    if (i < text.length && (text.data[i] == '-' || text.data[i] == '+')) negative = text.data[i++] == '-';
    if (i == text.length) cx_fail("DOMAIN", "string is not a signed integer", location);
    uint64_t magnitude = 0, limit = negative ? (UINT64_C(1) << 63) : INT64_MAX;
    for (; i < text.length; ++i) {
        unsigned digit = (unsigned char)text.data[i] - (unsigned)'0';
        if (digit > 9 || magnitude > (limit - digit) / 10)
            cx_fail("DOMAIN", "string is not a signed integer", location);
        magnitude = magnitude * 10 + digit;
    }
    if (!negative) return (int64_t)magnitude;
    return magnitude == (UINT64_C(1) << 63) ? INT64_MIN : -(int64_t)magnitude;
}
static double cx_parse_float(cx_str text, cx_loc location) {
    size_t i = 0, digits = 0;
    if (i < text.length && (text.data[i] == '+' || text.data[i] == '-')) ++i;
    while (i < text.length && text.data[i] >= '0' && text.data[i] <= '9') { ++i; ++digits; }
    if (i < text.length && text.data[i] == '.') {
        ++i;
        while (i < text.length && text.data[i] >= '0' && text.data[i] <= '9') { ++i; ++digits; }
    }
    if (!digits) cx_fail("DOMAIN", "string is not a float", location);
    if (i < text.length && (text.data[i] == 'e' || text.data[i] == 'E')) {
        ++i;
        if (i < text.length && (text.data[i] == '+' || text.data[i] == '-')) ++i;
        size_t start = i;
        while (i < text.length && text.data[i] >= '0' && text.data[i] <= '9') ++i;
        if (start == i) cx_fail("DOMAIN", "string is not a float", location);
    }
    if (i != text.length) cx_fail("DOMAIN", "string is not a float", location);
    /* Supported strings are NUL-terminated literals or process arguments. */
    return cx_finite(strtod(text.data, NULL), location);
}
static int64_t cx_float_to_int(double value, cx_loc location) {
    if (!(value >= -0x1p63 && value < 0x1p63))
        cx_fail("DOMAIN", "to_int value out of range or unsupported", location);
    return (int64_t)value;
}
static int64_t cx_uint_to_int(uint64_t value, cx_loc location) {
    if (value > INT64_MAX) cx_fail("DOMAIN", "unsigned value exceeds int range", location);
    return (int64_t)value;
}

enum cx_kind { CX_INT, CX_UINT, CX_FLOAT, CX_BOOL, CX_CHAR, CX_STRING, CX_VOID, CX_FUNCTION };
typedef struct {
    enum cx_kind kind;
    union { int64_t i; uint64_t u; double f; bool b; uint32_t c; cx_str s; } as;
} cx_value;
typedef struct { char *data; size_t length, capacity; } cx_buffer;
static void cx_append(cx_buffer *out, const char *data, size_t length, cx_loc location) {
    if (length > CX_LIMIT - out->length) cx_fail("LIMIT", "captured output exceeds 8 MiB", location);
    size_t needed = out->length + length;
    if (needed > out->capacity) {
        size_t capacity = out->capacity ? out->capacity : 128;
        while (capacity < needed) capacity *= 2;
        char *next = (char *)realloc(out->data, capacity);
        if (!next) cx_fail("LIMIT", "native allocation failed", location);
        out->data = next;
        out->capacity = capacity;
    }
    if (length) memcpy(out->data + out->length, data, length);
    out->length = needed;
}
static void cx_character(cx_buffer *out, uint32_t value, cx_loc location) {
    char data[4];
    size_t length;
    if (value < 0x80) { data[0] = (char)value; length = 1; }
    else if (value < 0x800) {
        data[0] = (char)(0xc0 | (value >> 6)); data[1] = (char)(0x80 | (value & 63)); length = 2;
    } else if (value < 0x10000) {
        data[0] = (char)(0xe0 | (value >> 12)); data[1] = (char)(0x80 | ((value >> 6) & 63));
        data[2] = (char)(0x80 | (value & 63)); length = 3;
    } else {
        data[0] = (char)(0xf0 | (value >> 18)); data[1] = (char)(0x80 | ((value >> 12) & 63));
        data[2] = (char)(0x80 | ((value >> 6) & 63)); data[3] = (char)(0x80 | (value & 63)); length = 4;
    }
    cx_append(out, data, length, location);
}
static void cx_value_text(cx_buffer *out, cx_value value, cx_loc location) {
    char buffer[512];
    int length;
    switch (value.kind) {
        case CX_INT: length = snprintf(buffer, sizeof buffer, "%" PRId64, value.as.i); break;
        case CX_UINT: length = snprintf(buffer, sizeof buffer, "%" PRIu64, value.as.u); break;
        case CX_FLOAT:
            if (isnan(value.as.f)) { cx_append(out, "NaN", 3, location); return; }
            if (isinf(value.as.f)) {
                const char *text = signbit(value.as.f) ? "-inf" : "inf";
                cx_append(out, text, strlen(text), location); return;
            }
            cx_append(out, buffer, cx_format_float(value.as.f, buffer), location); return;
        case CX_BOOL:
            cx_append(out, value.as.b ? "verified" : "unverified", value.as.b ? 8 : 10, location); return;
        case CX_CHAR: cx_character(out, value.as.c, location); return;
        case CX_STRING: case CX_FUNCTION: cx_append(out, value.as.s.data, value.as.s.length, location); return;
        case CX_VOID: cx_append(out, "void", 4, location); return;
        default: cx_fail("DOMAIN", "invalid native value", location);
    }
    if (length < 0 || (size_t)length >= sizeof buffer) cx_fail("FORMAT", "number formatting failed", location);
    cx_append(out, buffer, (size_t)length, location);
}
#ifdef _WIN32
static bool cx_write_console(const cx_buffer *out, cx_loc location) {
    HANDLE handle = (HANDLE)_get_osfhandle(_fileno(stdout));
    DWORD mode;
    if (!GetConsoleMode(handle, &mode)) return false;
    if (!out->length) return true;
    int length = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, out->data, (int)out->length, NULL, 0);
    if (!length) cx_fail("IO", "cannot decode native console output as UTF-8", location);
    WCHAR *wide = (WCHAR *)malloc((size_t)length * sizeof(WCHAR));
    if (!wide) cx_fail("LIMIT", "cannot allocate native console output", location);
    if (!MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, out->data, (int)out->length, wide, length)) {
        free(wide);
        cx_fail("IO", "cannot convert native console output to Unicode", location);
    }
    for (int offset = 0; offset < length;) {
        DWORD chunk = (DWORD)(length - offset);
        if (chunk > 32768) chunk = 32768;
        /* Do not split a surrogate pair between normal console writes. */
        WCHAR last = wide[offset + chunk - 1];
        if (chunk < (DWORD)(length - offset) && last >= 0xd800 && last <= 0xdbff) --chunk;
        DWORD written = 0;
        if (!WriteConsoleW(handle, wide + offset, chunk, &written, NULL) || !written || written > chunk) {
            free(wide);
            cx_fail("IO", "cannot write Unicode to standard output", location);
        }
        offset += (int)written;
    }
    free(wide);
    return true;
}
#endif
static void cx_flush(cx_buffer *out, cx_loc location) {
    if (out->length > CX_LIMIT - cx_output_size) cx_fail("LIMIT", "captured output exceeds 8 MiB", location);
#ifdef _WIN32
    if (!cx_write_console(out, location))
#endif
    if (out->length && fwrite(out->data, 1, out->length, stdout) != out->length)
        cx_fail("IO", "cannot write standard output", location);
    if (fflush(stdout)) cx_fail("IO", "cannot flush standard output", location);
    cx_output_size += out->length;
    free(out->data);
}
static void cx_jole(size_t count, const cx_value *args, cx_loc location) {
    cx_buffer out = {0};
    if (!count) cx_append(&out, "jole", 4, location);
    for (size_t i = 0; i < count; ++i) {
        if (i) cx_append(&out, " ", 1, location);
        cx_value_text(&out, args[i], location);
    }
    if (!out.length || out.data[out.length - 1] != '\n') cx_append(&out, "\n", 1, location);
    cx_flush(&out, location);
}
static void cx_yap(cx_str format, size_t count, const cx_value *args, cx_loc location) {
    cx_buffer out = {0};
    size_t argument = 0;
    for (size_t i = 0; i < format.length;) {
        char ch = format.data[i++];
        if (ch != '%') { cx_append(&out, &ch, 1, location); continue; }
        if (i < format.length && format.data[i] == '%') { ++i; cx_append(&out, "%", 1, location); continue; }
        size_t width = 0;
        while (i < format.length && format.data[i] >= '0' && format.data[i] <= '9') {
            unsigned digit = (unsigned)(format.data[i++] - '0');
            if (width > 1000) cx_fail("FORMAT", "format width exceeds 1000", location);
            width = width * 10 + digit;
        }
        if (width > 1000) cx_fail("FORMAT", "format width exceeds 1000", location);
        unsigned precision = 6;
        if (i < format.length && format.data[i] == '.') {
            ++i;
            precision = 0;
            size_t start = i;
            while (i < format.length && format.data[i] >= '0' && format.data[i] <= '9') {
                if (precision > 15) cx_fail("FORMAT", "precision exceeds 15", location);
                precision = precision * 10 + (unsigned)(format.data[i++] - '0');
            }
            if (start == i) cx_fail("FORMAT", "invalid precision", location);
            if (precision > 15) cx_fail("FORMAT", "precision exceeds 15", location);
        }
        if (i == format.length) cx_fail("FORMAT", "unfinished percent format", location);
        char spec = format.data[i++];
        if (argument == count) cx_fail("FORMAT", "not enough format arguments", location);
        cx_value value = args[argument++];
        bool numeric = value.kind == CX_INT || value.kind == CX_UINT || value.kind == CX_FLOAT;
        bool valid = spec == 'v' || (spec == 'd' && value.kind == CX_INT)
            || (spec == 'u' && value.kind == CX_UINT) || (spec == 'f' && numeric)
            || (spec == 's' && value.kind == CX_STRING) || (spec == 'c' && value.kind == CX_CHAR)
            || (spec == 'b' && value.kind == CX_BOOL);
        if (!valid) cx_fail("FORMAT", "format specifier does not accept argument type", location);
        cx_buffer field = {0};
        if (spec == 'f') {
            double number = value.kind == CX_INT ? (double)value.as.i : value.kind == CX_UINT ? (double)value.as.u : value.as.f;
            char text[512];
            int length;
            if (isnan(number)) length = snprintf(text, sizeof text, "NaN");
            else length = snprintf(text, sizeof text, "%.*f", (int)precision, number);
            if (length < 0 || (size_t)length >= sizeof text) cx_fail("FORMAT", "float formatting failed", location);
            cx_append(&field, text, (size_t)length, location);
        } else cx_value_text(&field, value, location);
        size_t characters = cx_chars((cx_str){field.data, field.length});
        while (characters++ < width) cx_append(&out, " ", 1, location);
        cx_append(&out, field.data, field.length, location);
        free(field.data);
    }
    if (argument != count) cx_fail("FORMAT", "too many format arguments", location);
    cx_flush(&out, location);
}
static void cx_assert(bool condition, bool has_message, cx_value message, cx_loc location) {
    if (condition) return;
    if (!has_message) cx_fail("ASSERT", "assertion failed", location);
    cx_buffer text = {0};
    cx_value_text(&text, message, location);
    fprintf(stderr, "error[ASSERT]: ");
    if (text.length) fwrite(text.data, 1, text.length, stderr);
    fprintf(stderr, "\n --> %s:%zu:%zu\n", location.file, location.line, location.column);
    free(text.data);
    exit(1);
}
static cx_str cx_argument(int64_t index, cx_loc location) {
    if (index < 0 || index >= cx_arg_count) cx_fail("DOMAIN", "argument index out of bounds", location);
    const char *text = cx_args[index];
    return (cx_str){text, strlen(text)};
}
