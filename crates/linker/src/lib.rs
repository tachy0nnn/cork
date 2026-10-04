#![allow(unsafe_code)]
pub mod elf;
pub mod resolver;
pub mod shims;

use jni::JniVm;
use resolver::SymbolResolver;
use std::ffi::{CStr, CString, c_void};
use std::path::Path;

pub struct AndroidLoader {
    vm: JniVm,
    resolver: SymbolResolver,
}

impl AndroidLoader {
    /// initialize a new loader with an active fake JVM
    pub fn new() -> Result<Self, &'static str> {
        let vm = JniVm::new()?;
        let resolver = SymbolResolver::new();
        Ok(Self { vm, resolver })
    }

    /// set search directories for Android Bionic library resolution
    pub fn set_search_path(&self, path: &Path) {
        let path_str = path.to_string_lossy();
        if let Ok(c_path) = CString::new(path_str.as_bytes()) {
            unsafe {
                resolver::cork_linker_set_search_path(c_path.as_ptr());
            }

            for dir in path_str.split(':') {
                if !dir.is_empty() {
                    self.resolver.scan_and_resolve_dir(Path::new(dir));
                }
            }
        }
    }

    /// load an android native library (.so) and invoke its JNI_OnLoad
    pub fn load_library(&self, path: &Path) -> Result<i32, String> {
        self.resolver.scan_and_resolve_file(path);

        let c_path = CString::new(path.to_string_lossy().as_bytes())
            .map_err(|e| format!("Invalid library path: {e}"))?;

        unsafe {
            // open the dynamic library
            let handle = resolver::cork_linker_load_library(c_path.as_ptr());

            if handle.is_null() {
                let err = resolver::cork_linker_get_error();
                let err_msg = if err.is_null() {
                    "Unknown linker error".to_string()
                } else {
                    CStr::from_ptr(err).to_string_lossy().into_owned()
                };
                return Err(format!(
                    "mcpelauncher-linker failed to load {}: {err_msg}",
                    path.display()
                ));
            }

            // call JNI_OnLoad with the fake JVM
            let version = resolver::cork_linker_call_jni_onload(handle, self.vm.java_vm());
            Ok(version)
        }
    }

    /// direct access to the underlying JniVm
    #[must_use]
    pub fn vm(&self) -> &JniVm {
        &self.vm
    }

    /// retrieve a symbol from a loaded library handle
    #[must_use]
    pub unsafe fn get_symbol(&self, handle: *mut c_void, name: &str) -> *mut c_void {
        if let Ok(c_name) = CString::new(name) {
            unsafe { resolver::cork_linker_get_symbol(handle, c_name.as_ptr()) }
        } else {
            std::ptr::null_mut()
        }
    }
}
