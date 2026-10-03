#![allow(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
use jni::JniVm;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::path::Path;

// signature of android's JNI_OnLoad entry point
type JniOnLoadFn = unsafe extern "C" fn(vm: *mut c_void, reserved: *mut c_void) -> c_int;

unsafe extern "C" {
    fn cork_linker_load_library(path: *const c_char) -> *mut c_void;
    fn cork_linker_get_symbol(handle: *mut c_void, symbol_name: *const c_char) -> *mut c_void;
    fn cork_linker_get_error() -> *const c_char;
    fn cork_linker_set_search_path(path: *const c_char);
}

pub struct AndroidLoader {
    vm: JniVm,
}

impl AndroidLoader {
    /// initialize a new loader with an active fake JVM
    pub fn new() -> Result<Self, &'static str> {
        let vm = JniVm::new()?;
        Ok(Self { vm })
    }

    /// set search directories for Android Bionic library resolution
    pub fn set_search_path(&self, path: &Path) {
        if let Ok(c_path) = CString::new(path.to_str().unwrap_or_default()) {
            unsafe { cork_linker_set_search_path(c_path.as_ptr()) };
        }
    }

    /// load an android native library (.so) and invoke its JNI_OnLoad
    pub fn load_library(&self, so_path: &Path) -> Result<i32, String> {
        let c_path =
            CString::new(so_path.to_str().ok_or("Invalid path")?).map_err(|e| e.to_string())?;

        unsafe {
            // open the dynamic library
            let handle = cork_linker_load_library(c_path.as_ptr());
            if handle.is_null() {
                let err_ptr = cork_linker_get_error();
                let err_msg = if err_ptr.is_null() {
                    "unknown linker error".into()
                } else {
                    CStr::from_ptr(err_ptr).to_string_lossy().into_owned()
                };
                return Err(format!(
                    "mcpelauncher-linker failed to load {}: {}",
                    so_path.display(),
                    err_msg
                ));
            }

            // look up JNI_OnLoad
            let sym_name = CString::new("JNI_OnLoad").unwrap();
            let sym = cork_linker_get_symbol(handle, sym_name.as_ptr());

            if sym.is_null() {
                // some libraries don't export JNI_OnLoad
                return Ok(0);
            }

            // call JNI_OnLoad with the fake JVM
            let on_load: JniOnLoadFn = std::mem::transmute(sym);
            let version = on_load(self.vm.java_vm(), std::ptr::null_mut());

            Ok(version)
        }
    }

    /// direct access to the underlying JniVm
    #[must_use]
    pub fn vm(&self) -> &JniVm {
        &self.vm
    }
}
