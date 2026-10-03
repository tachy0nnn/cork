#include <string>
#include <memory>
#include <unordered_map>
#include <vector>
#include <forward_list>

#ifndef EnableJNIVMGC
#define EnableJNIVMGC
#endif

#include <jnivm/env.h>
#include <jnivm.h>
#include "bridge.h"

struct CorkVM {
    jnivm::VM vm;
};

extern "C" {

CorkVM* cork_vm_create(void) {
    return new CorkVM();
}

void cork_vm_destroy(CorkVM* vm) {
    if (vm != nullptr) {
        delete vm;
    }
}

void* cork_vm_get_javavm(CorkVM* vm) {
    if (vm == nullptr) return nullptr;
    return static_cast<void*>(vm->vm.GetJavaVM());
}

void* cork_vm_get_jnienv(CorkVM* vm) {
    if (vm == nullptr) return nullptr;
    return static_cast<void*>(vm->vm.GetJNIEnv());
}

}