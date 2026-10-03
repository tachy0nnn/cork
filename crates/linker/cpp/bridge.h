#ifndef CORK_LINKER_BRIDGE_H
#define CORK_LINKER_BRIDGE_H

#ifdef __cplusplus
extern "C" {
#endif

void* cork_linker_load_library(const char* path);
void* cork_linker_get_symbol(void* handle, const char* symbol_name);
const char* cork_linker_get_error(void);
void cork_linker_set_search_path(const char* path);

#ifdef __cplusplus
}
#endif

#endif // CORK_LINKER_BRIDGE_H