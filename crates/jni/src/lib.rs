#![allow(unsafe_code)] // welp i have no choice, but to suppress warnings :P
use std::ffi::c_void;
use std::ptr::NonNull;

// raw FFI declarations
#[repr(C)]
struct CorkVM {
    _private: [u8; 0],
}

unsafe extern "C" {
    fn cork_vm_create() -> *mut CorkVM;
    fn cork_vm_destroy(vm: *mut CorkVM);
    fn cork_vm_get_javavm(vm: *mut CorkVM) -> *mut c_void;
    fn cork_vm_get_jnienv(vm: *mut CorkVM) -> *mut c_void;
}

/// wrapper around libjnivm
pub struct JniVm {
    handle: NonNull<CorkVM>,
}

unsafe impl Send for JniVm {}

impl JniVm {
    /// initialize a new fake JVM
    pub fn new() -> Result<Self, &'static str> {
        let raw = unsafe { cork_vm_create() };
        NonNull::new(raw)
            .map(|handle| Self { handle })
            .ok_or("Failed to initialize libjnivm instance")
    }

    /// retrieve the raw `JavaVM*` pointer
    pub fn java_vm(&self) -> *mut c_void {
        unsafe { cork_vm_get_javavm(self.handle.as_ptr()) }
    }

    /// retrieve the raw `JNIEnv*` pointer
    pub fn jni_env(&self) -> *mut c_void {
        unsafe { cork_vm_get_jnienv(self.handle.as_ptr()) }
    }
}

impl Drop for JniVm {
    fn drop(&mut self) {
        unsafe {
            cork_vm_destroy(self.handle.as_ptr());
        }
    }
}
