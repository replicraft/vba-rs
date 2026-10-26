/* Minimal startup object for the test-only Wine helper build.
 *
 * Stands in for the pieces of the Microsoft C runtime that rustc's
 * x86_64-pc-windows-msvc output expects when no MSVC/SDK libraries exist:
 * the process entry point, the TLS directory and one C++ vftable symbol.
 * Not for any other use. */
#include <stdint.h>

extern int main(int, char **);
extern void ExitProcess(unsigned);

/* Entry point: Rust's generated `main` reads the command line itself. */
void mainCRTStartup(void) { ExitProcess((unsigned)main(0, 0)); }

/* TLS directory (`_tls_used`) and the slot index the loader fills in. */
unsigned _tls_index = 0;

/* Markers bracketing the thread-local data template: the compiler places
 * `#[thread_local]` data in sections named `.tls$...`, which sort between
 * `.tls` and `.tls$ZZZ`. */
#pragma section(".tls", read, write)
#pragma section(".tls$ZZZ", read, write)
__declspec(allocate(".tls")) char _tls_start = 0;
__declspec(allocate(".tls$ZZZ")) char _tls_end = 0;

/* Empty callback list (null terminator only). */
#pragma section(".CRT$XLZ", read)
__declspec(allocate(".CRT$XLZ")) void *_tls_callbacks_end = 0;

typedef struct {
  uint64_t start, end, index, callbacks;
  uint32_t zero_fill, characteristics;
} TlsDir;

#pragma section(".rdata$T", read)
__declspec(allocate(".rdata$T")) const TlsDir _tls_used = {
    (uint64_t)&_tls_start, (uint64_t)&_tls_end,
    (uint64_t)&_tls_index, (uint64_t)&_tls_callbacks_end, 0, 0};

/* Referenced by the standard library's exception plumbing; with panic=abort it
 * is never dereferenced. */
const void *const type_info_vftable[2] __asm__("??_7type_info@@6B@") = {0, 0};
