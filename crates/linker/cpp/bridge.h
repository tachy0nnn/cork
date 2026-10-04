#pragma once

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

void cork_linker_init(void);
void* cork_linker_load_virtual_library(
    const char* name,
    const char* const* sym_names,
    void* const* sym_addrs,
    size_t count
);
void cork_linker_relocate_virtual_library(
    void* handle,
    const char* const* sym_names,
    void* const* sym_addrs,
    size_t count
);
void* cork_linker_load_library(const char* path);
void* cork_linker_get_symbol(void* handle, const char* symbol_name);
const char* cork_linker_get_error(void);
void cork_linker_set_search_path(const char* path);
int cork_linker_call_jni_onload(void* handle, void* java_vm);

#ifdef __cplusplus
}
#endif