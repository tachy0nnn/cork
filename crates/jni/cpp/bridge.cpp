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
    JavaVM exposed_vm;
    JavaVM clean_vm;
    JNIInvokeInterface clean_interface;
    JNIInvokeInterface wrapped_interface;

    CorkVM() {
        JavaVM* base_vm = vm.GetJavaVM();
        clean_interface = *base_vm->functions;
        clean_vm.functions = &clean_interface;

        wrapped_interface = clean_interface;

        wrapped_interface.DestroyJavaVM = [](JavaVM* vm) -> jint {
            CorkVM* self = get_cork_vm(vm);
            return self->clean_interface.DestroyJavaVM(&self->clean_vm);
        };

        wrapped_interface.AttachCurrentThread = [](JavaVM* vm, JNIEnv** penv, void* args) -> jint {
            CorkVM* self = get_cork_vm(vm);
            if (vm && vm->functions && vm->functions->reserved0 == nullptr) {
                ((void**)vm->functions)[0] = self->clean_interface.reserved0;
            }
            return self->clean_interface.AttachCurrentThread(&self->clean_vm, penv, args);
        };

        wrapped_interface.DetachCurrentThread = [](JavaVM* vm) -> jint {
            CorkVM* self = get_cork_vm(vm);
            if (vm && vm->functions && vm->functions->reserved0 == nullptr) {
                ((void**)vm->functions)[0] = self->clean_interface.reserved0;
            }
            return self->clean_interface.DetachCurrentThread(&self->clean_vm);
        };

        wrapped_interface.GetEnv = [](JavaVM* vm, void** penv, jint ver) -> jint {
            CorkVM* self = get_cork_vm(vm);
            if (vm && vm->functions && vm->functions->reserved0 == nullptr) {
                ((void**)vm->functions)[0] = self->clean_interface.reserved0;
            }
            return self->clean_interface.GetEnv(&self->clean_vm, penv, ver);
        };

        wrapped_interface.AttachCurrentThreadAsDaemon = [](JavaVM* vm, JNIEnv** penv, void* args) -> jint {
            CorkVM* self = get_cork_vm(vm);
            if (vm && vm->functions && vm->functions->reserved0 == nullptr) {
                ((void**)vm->functions)[0] = self->clean_interface.reserved0;
            }
            return self->clean_interface.AttachCurrentThreadAsDaemon(&self->clean_vm, penv, args);
        };

        exposed_vm.functions = &wrapped_interface;
        s_instance = this;
    }

    ~CorkVM() {
        if (s_instance == this) {
            s_instance = nullptr;
        }
    }

    static CorkVM* s_instance;
    static CorkVM* get_cork_vm(JavaVM* /*vm*/) {
        return s_instance;
    }
};

CorkVM* CorkVM::s_instance = nullptr;

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
    return static_cast<void*>(&vm->exposed_vm);
}

void* cork_vm_get_jnienv(CorkVM* vm) {
    if (vm == nullptr) return nullptr;
    return static_cast<void*>(vm->vm.GetJNIEnv());
}

}