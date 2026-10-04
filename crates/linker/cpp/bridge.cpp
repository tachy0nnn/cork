#include "bridge.h"
#include <mcpelauncher/linker.h>
#include <unordered_map>
#include <unordered_set>
#include <string>
#include <link.h>

// weak symbol stubs for GNU ld compatibility
#if defined(USE_RELA) || defined(__x86_64__) || defined(__aarch64__)
__attribute__((weak)) ElfW(Rela) __rela_iplt_start[1] = {};
__attribute__((weak)) ElfW(Rela) __rela_iplt_end[1] = {};
#else
__attribute__((weak)) ElfW(Rel) __rel_iplt_start[1] = {};
__attribute__((weak)) ElfW(Rel) __rel_iplt_end[1] = {};
#endif

extern "C" {

void cork_linker_init(void) {
    linker::init();
}

void* cork_linker_load_virtual_library(
    const char* name,
    const char* const* sym_names,
    void* const* sym_addrs,
    size_t count
) {
    if (!name) return nullptr;
    static std::unordered_set<std::string> s_library_names;
    const char* permanent_name = s_library_names.insert(name).first->c_str();

    std::unordered_map<std::string, void*> syms;
    syms.reserve(count);
    for (size_t i = 0; i < count; i++) {
        syms[sym_names[i]] = sym_addrs[i];
    }
    void* ret = linker::load_library(permanent_name, syms);
    return ret;
}

void cork_linker_relocate_virtual_library(
    void* handle,
    const char* const* sym_names,
    void* const* sym_addrs,
    size_t count
) {
    if (!handle || count == 0) return;
    std::unordered_map<std::string, void*> syms;
    syms.reserve(count);
    for (size_t i = 0; i < count; i++) {
        syms[sym_names[i]] = sym_addrs[i];
    }
    linker::relocate(handle, syms);
}

void* cork_linker_load_library(const char* path) {
    if (path == nullptr) return nullptr;
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
        __loader_android_update_LD_LIBRARY_PATH(path);
    }
}

int cork_linker_call_jni_onload(void* handle, void* java_vm) {
    if (!handle || !java_vm) return 0;
    typedef int (*JniOnLoadFn)(void* vm, void* reserved);
    JniOnLoadFn on_load = (JniOnLoadFn)linker::dlsym(handle, "JNI_OnLoad");
    if (!on_load) return 0;

    return on_load(java_vm, nullptr);
}

}