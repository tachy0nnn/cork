#![allow(unsafe_code)]
use std::ffi::{c_int, c_void, CStr, CString};
use std::path::Path;
use jni::JniVm;

// signature of android's JNI_OnLoad entry point
type JniOnLoadFn = unsafe extern "C" fn(vm: *mut c_void, reserved: *mut c_void) -> c_int;

pub struct AndroidLoader {
    vm: JniVm,
}

impl AndroidLoader {
    /// initialize a new loader with an active fake JVM
    pub fn new() -> Result<Self, &'static str> {
        let vm = JniVm::new()?;
        Ok(Self { vm })
    }

    /// load an android native library (.so) and invoke its JNI_OnLoad
    pub fn load_library(&self, so_path: &Path) -> Result<i32, String> {
        let c_path = CString::new(so_path.to_str().ok_or("Invalid path")?)
            .map_err(|e| e.to_string())?;

        unsafe {
            // open the dynamic library
            let handle = libc::dlopen(c_path.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL);
            if handle.is_null() {
                let err_ptr = libc::dlerror();
                let err_msg = if !err_ptr.is_null() {
                    CStr::from_ptr(err_ptr).to_string_lossy().into_owned()
                } else {
                    "unknown dlopen error".into()
                };
                return Err(format!("Failed to dlopen {}: {}", so_path.display(), err_msg));
            }

            // look up JNI_OnLoad
            let sym_name = CString::new("JNI_OnLoad").unwrap();
            let sym = libc::dlsym(handle, sym_name.as_ptr());

            if sym.is_null() {
                // some libraries don't export JNI_OnLoad
                return Ok(0);
            }

            // call JNI_OnLoad with the fake JVM pointer
            let on_load: JniOnLoadFn = std::mem::transmute(sym);
            let jni_version = on_load(self.vm.java_vm(), std::ptr::null_mut());

            Ok(jni_version)
        }
    }

    /// direct access to the underlying JniVm
    pub fn vm(&self) -> &JniVm {
        &self.vm
    }
}