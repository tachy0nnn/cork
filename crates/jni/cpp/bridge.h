#ifndef CORK_JNIVM_BRIDGE_H
#define CORK_JNIVM_BRIDGE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// handle to the C++ VM instance
typedef struct CorkVM CorkVM;

// lifecycle
CorkVM* cork_vm_create(void);
void cork_vm_destroy(CorkVM* vm);

// JNI pointers to pass to android libraries (JNI_OnLoad)
void* cork_vm_get_javavm(CorkVM* vm);
void* cork_vm_get_jnienv(CorkVM* vm);

#ifdef __cplusplus
}
#endif

#endif // CORK_JNIVM_BRIDGE_H