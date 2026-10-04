use std::collections::{HashMap, HashSet};
use std::ffi::{CString, c_char, c_int, c_void};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use crate::elf;
use crate::shims;

unsafe extern "C" {
    pub fn cork_linker_init();
    pub fn cork_linker_load_virtual_library(
        name: *const c_char,
        sym_names: *const *const c_char,
        sym_addrs: *const *mut c_void,
        count: usize,
    ) -> *mut c_void;
    pub fn cork_linker_relocate_virtual_library(
        handle: *mut c_void,
        sym_names: *const *const c_char,
        sym_addrs: *const *mut c_void,
        count: usize,
    );
    pub fn cork_linker_load_library(path: *const c_char) -> *mut c_void;
    pub fn cork_linker_get_symbol(handle: *mut c_void, symbol_name: *const c_char) -> *mut c_void;
    pub fn cork_linker_get_error() -> *const c_char;
    pub fn cork_linker_set_search_path(path: *const c_char);
    pub fn cork_linker_call_jni_onload(handle: *mut c_void, java_vm: *mut c_void) -> c_int;
}

pub struct SymbolResolver {
    h_libc: *mut c_void,
    h_libm: *mut c_void,
    h_libz: *mut c_void,
    h_libegl: *mut c_void,
    h_libgles: *mut c_void,

    vl_libc: *mut c_void,
    vl_libm: *mut c_void,
    vl_libz: *mut c_void,
    vl_libegl: *mut c_void,
    vl_libgles: *mut c_void,

    resolved_symbols: Mutex<HashSet<String>>,
}

unsafe impl Send for SymbolResolver {}
unsafe impl Sync for SymbolResolver {}

impl SymbolResolver {
    pub fn new() -> Self {
        unsafe {
            cork_linker_init();
        }

        let mut resolved = HashSet::new();

        // open host shared libraries
        let h_libc = open_host_lib(&["libc.so.6", "libc.so"]);
        let h_libm = open_host_lib(&["libm.so.6", "libm.so"]);
        let h_libz = open_host_lib(&["libz.so.1", "libz.so"]);
        let h_libegl = open_host_lib(&["libEGL.so.1", "libEGL.so"]);
        let h_libgles = open_host_lib(&["libGLESv2.so.2", "libGLESv2.so"]);

        // base libc shims
        let libc_shims = shims::get_libc_shims();
        for (name, _) in &libc_shims {
            resolved.insert((*name).to_string());
        }
        let vl_libc = unsafe { register_virtual_library("libc.so", &libc_shims) };

        // empty shims for now (FOR NOW :P)
        let vl_libm = unsafe { register_virtual_library("libm.so", &[]) };
        let vl_libz = unsafe { register_virtual_library("libz.so", &[]) };
        let vl_libegl = unsafe { register_virtual_library("libEGL.so", &[]) };
        let vl_libgles = unsafe { register_virtual_library("libGLESv2.so", &[]) };

        // android NDK libraries
        let log_shims = shims::get_liblog_shims();
        for (name, _) in &log_shims {
            resolved.insert((*name).to_string());
        }
        let _ = unsafe { register_virtual_library("liblog.so", &log_shims) };

        let android_shims = shims::get_libandroid_shims();
        for (name, _) in &android_shims {
            resolved.insert((*name).to_string());
        }
        let _ = unsafe { register_virtual_library("libandroid.so", &android_shims) };

        let opensles_shims = shims::get_opensles_shims();
        for (name, _) in &opensles_shims {
            resolved.insert((*name).to_string());
        }
        let _ = unsafe { register_virtual_library("libOpenSLES.so", &opensles_shims) };

        let openmaxal_shims = shims::get_openmaxal_shims();
        for (name, _) in &openmaxal_shims {
            resolved.insert((*name).to_string());
        }
        let _ = unsafe { register_virtual_library("libOpenMAXAL.so", &openmaxal_shims) };

        let mediandk_shims = shims::get_mediandk_shims();
        for (name, _) in &mediandk_shims {
            resolved.insert((*name).to_string());
        }
        let _ = unsafe { register_virtual_library("libmediandk.so", &mediandk_shims) };

        let jnigraphics_shims = shims::get_jnigraphics_shims();
        for (name, _) in &jnigraphics_shims {
            resolved.insert((*name).to_string());
        }
        let _ = unsafe { register_virtual_library("libjnigraphics.so", &jnigraphics_shims) };

        Self {
            h_libc,
            h_libm,
            h_libz,
            h_libegl,
            h_libgles,
            vl_libc,
            vl_libm,
            vl_libz,
            vl_libegl,
            vl_libgles,
            resolved_symbols: Mutex::new(resolved),
        }
    }

    pub fn resolve_symbols(&self, symbols: &HashSet<String>) {
        let mut resolved = self.resolved_symbols.lock().unwrap();

        let mut new_libc = HashMap::new();
        let mut new_libm = HashMap::new();
        let mut new_libz = HashMap::new();
        let mut new_libegl = HashMap::new();
        let mut new_libgles = HashMap::new();

        for sym in symbols {
            if resolved.contains(sym) {
                continue;
            }

            let Ok(c_sym) = CString::new(sym.as_str()) else {
                continue;
            };

            // graphics GLES
            if sym.starts_with("gl") && !self.h_libgles.is_null() {
                let s = unsafe { libc::dlsym(self.h_libgles, c_sym.as_ptr()) };
                if !s.is_null() {
                    new_libgles.insert(sym.clone(), s);
                    resolved.insert(sym.clone());
                    continue;
                }
            }

            // graphics EGL
            if sym.starts_with("egl") && !self.h_libegl.is_null() {
                let s = unsafe { libc::dlsym(self.h_libegl, c_sym.as_ptr()) };
                if !s.is_null() {
                    new_libegl.insert(sym.clone(), s);
                    resolved.insert(sym.clone());
                    continue;
                }
            }

            // compression (zlib)
            if (sym.starts_with("inflate")
                || sym.starts_with("deflate")
                || sym.starts_with("crc32")
                || sym.starts_with("adler32")
                || sym.starts_with("zlib"))
                && !self.h_libz.is_null()
            {
                let s = unsafe { libc::dlsym(self.h_libz, c_sym.as_ptr()) };
                if !s.is_null() {
                    new_libz.insert(sym.clone(), s);
                    resolved.insert(sym.clone());
                    continue;
                }
            }

            // math (libm)
            if !self.h_libm.is_null() {
                let s = unsafe { libc::dlsym(self.h_libm, c_sym.as_ptr()) };
                if !s.is_null() {
                    new_libm.insert(sym.clone(), s);
                    new_libc.insert(sym.clone(), s);
                    resolved.insert(sym.clone());
                    continue;
                }
            }

            // host libc & RTLD_DEFAULT
            let mut s = if self.h_libc.is_null() {
                std::ptr::null_mut()
            } else {
                unsafe { libc::dlsym(self.h_libc, c_sym.as_ptr()) }
            };
            if s.is_null() {
                s = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c_sym.as_ptr()) };
            }
            if !s.is_null() {
                new_libc.insert(sym.clone(), s);
                resolved.insert(sym.clone());
            }
        }

        unsafe {
            if !new_libc.is_empty() {
                relocate_virtual_library(self.vl_libc, &new_libc);
            }
            if !new_libm.is_empty() {
                relocate_virtual_library(self.vl_libm, &new_libm);
            }
            if !new_libz.is_empty() {
                relocate_virtual_library(self.vl_libz, &new_libz);
            }
            if !new_libegl.is_empty() {
                relocate_virtual_library(self.vl_libegl, &new_libegl);
            }
            if !new_libgles.is_empty() {
                relocate_virtual_library(self.vl_libgles, &new_libgles);
            }
        }
    }

    pub fn scan_and_resolve_file(&self, path: &Path) {
        let und_syms = elf::get_undefined_symbols(path);
        if !und_syms.is_empty() {
            self.resolve_symbols(&und_syms);
        }
    }

    pub fn scan_and_resolve_dir(&self, dir: &Path) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        let mut all_und = HashSet::new();
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|ext| ext == "so") {
                let s = elf::get_undefined_symbols(&p);
                all_und.extend(s);
            }
        }

        if !all_und.is_empty() {
            self.resolve_symbols(&all_und);
        }
    }
}

fn open_host_lib(candidates: &[&str]) -> *mut c_void {
    for name in candidates {
        if let Ok(c_str) = CString::new(*name) {
            let handle =
                unsafe { libc::dlopen(c_str.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) };
            if !handle.is_null() {
                return handle;
            }
        }
    }
    std::ptr::null_mut()
}

unsafe fn register_virtual_library(name: &str, symbols: &[(&str, *mut c_void)]) -> *mut c_void {
    let Ok(c_name) = CString::new(name) else {
        return std::ptr::null_mut();
    };

    let c_sym_names: Vec<CString> = symbols
        .iter()
        .map(|(n, _)| CString::new(*n).unwrap())
        .collect();
    let sym_name_ptrs: Vec<*const c_char> = c_sym_names.iter().map(|s| s.as_ptr()).collect();
    let sym_addr_ptrs: Vec<*mut c_void> = symbols.iter().map(|(_, a)| *a).collect();

    unsafe {
        cork_linker_load_virtual_library(
            c_name.as_ptr(),
            sym_name_ptrs.as_ptr(),
            sym_addr_ptrs.as_ptr(),
            symbols.len(),
        )
    }
}

unsafe fn relocate_virtual_library(handle: *mut c_void, symbols: &HashMap<String, *mut c_void>) {
    if handle.is_null() || symbols.is_empty() {
        return;
    }

    let c_sym_names: Vec<CString> = symbols
        .keys()
        .map(|k| CString::new(k.as_str()).unwrap())
        .collect();
    let sym_name_ptrs: Vec<*const c_char> = c_sym_names.iter().map(|s| s.as_ptr()).collect();
    let sym_addr_ptrs: Vec<*mut c_void> = symbols.values().copied().collect();

    unsafe {
        cork_linker_relocate_virtual_library(
            handle,
            sym_name_ptrs.as_ptr(),
            sym_addr_ptrs.as_ptr(),
            symbols.len(),
        );
    }
}

impl Default for SymbolResolver {
    fn default() -> Self {
        Self::new()
    }
}
