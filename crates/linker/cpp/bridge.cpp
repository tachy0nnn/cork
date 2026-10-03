#include "bridge.h"
#include <mcpelauncher/linker.h>
#include <mutex>
#include <link.h>

// weak symbol stubs for GNU ld compatibility
#if defined(USE_RELA) || defined(__x86_64__) || defined(__aarch64__)
__attribute__((weak)) ElfW(Rela) __rela_iplt_start[1] = {};
__attribute__((weak)) ElfW(Rela) __rela_iplt_end[1] = {};
#else
__attribute__((weak)) ElfW(Rel) __rel_iplt_start[1] = {};
__attribute__((weak)) ElfW(Rel) __rel_iplt_end[1] = {};
#endif

static std::once_flag g_linker_init_flag;

static void ensure_linker_initialized() {
    std::call_once(g_linker_init_flag, []() {
        linker::init();
    });
}

extern "C" {

void* cork_linker_load_library(const char* path) {
    if (path == nullptr) return nullptr;
    ensure_linker_initialized();
    // loads the library in-memory
    return linker::dlopen(path, 2);
}

void* cork_linker_get_symbol(void* handle, const char* symbol_name) {
    if (handle == nullptr || symbol_name == nullptr) return nullptr;
    return linker::dlsym(handle, symbol_name);
}

const char* cork_linker_get_error(void) {
    return __loader_dlerror();
}

void cork_linker_set_search_path(const char* path) {
    if (path != nullptr) {
        ensure_linker_initialized();
        __loader_android_update_LD_LIBRARY_PATH(path);
    }
}

}