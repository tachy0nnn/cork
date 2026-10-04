#include "bridge.h"
#include <mcpelauncher/linker.h>
#include <mutex>
#include <unordered_map>
#include <unordered_set>
#include <string>
#include <vector>
#include <cstdio>
#include <cstdlib>
#include <cstdarg>
#include <cstring>
#include <cerrno>
#include <unistd.h>
#include <dlfcn.h>
#include <link.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/socket.h>
#include <dirent.h>

// weak symbol stubs for GNU ld compatibility
#if defined(USE_RELA) || defined(__x86_64__) || defined(__aarch64__)
__attribute__((weak)) ElfW(Rela) __rela_iplt_start[1] = {};
__attribute__((weak)) ElfW(Rela) __rela_iplt_end[1] = {};
#else
__attribute__((weak)) ElfW(Rel) __rel_iplt_start[1] = {};
__attribute__((weak)) ElfW(Rel) __rel_iplt_end[1] = {};
#endif

static char bionic_sF[3][152] = {};
static uintptr_t bionic_stack_chk_guard = 0x595e9fbd94fda766ULL;

static FILE* unwrap_file(void* s) {
    if (s == (void*)&bionic_sF[0]) return stdin;
    if (s == (void*)&bionic_sF[1]) return stdout;
    if (s == (void*)&bionic_sF[2]) return stderr;
    return (FILE*)s;
}

static int shim_fflush(void* stream) {
    if (!stream) return fflush(nullptr);
    return fflush(unwrap_file(stream));
}

static int shim_fputs(const char* s, void* stream) {
    return fputs(s, unwrap_file(stream));
}

static size_t shim_fwrite(const void* ptr, size_t size, size_t nmemb, void* stream) {
    return fwrite(ptr, size, nmemb, unwrap_file(stream));
}

static size_t shim_fwrite_chk(const void* ptr, size_t size, size_t nmemb, void* stream, size_t) {
    return fwrite(ptr, size, nmemb, unwrap_file(stream));
}

static ssize_t shim_write_chk(int fd, const void* buf, size_t count, size_t) {
    return write(fd, buf, count);
}

static size_t shim_strlen_chk(const char* s, size_t) {
    return strlen(s);
}

static char* shim_strchr_chk(const char* s, int c, size_t) {
    return (char*)strchr(s, c);
}

static char* shim_strncpy_chk2(char* dst, const char* src, size_t n, size_t, size_t) {
    return strncpy(dst, src, n);
}

static mode_t shim_umask_chk(mode_t mask) {
    return umask(mask);
}

static void shim_FD_CLR_chk(int fd, fd_set* set, size_t) { FD_CLR(fd, set); }
static int shim_FD_ISSET_chk(int fd, fd_set* set, size_t) { return FD_ISSET(fd, set); }
static void shim_FD_SET_chk(int fd, fd_set* set, size_t) { FD_SET(fd, set); }

static ssize_t shim_sendto_chk(int s, const void* buf, size_t len, size_t, int flags, const struct sockaddr* to, socklen_t tolen) {
    return sendto(s, buf, len, flags, to, tolen);
}

static char* shim_gnu_strerror_r(int errnum, char* buf, size_t buflen) {
    return strerror_r(errnum, buf, buflen);
}

static int* shim_errno() {
    return &__errno_location()[0] ? &__errno_location()[0] : &errno;
}

static void shim_assert2(const char* file, int line, const char* func, const char* expr) {
    fprintf(stderr, "Assertion failed: %s (%s:%d: %s)\n", expr, file, line, func);
    abort();
}

static void shim_set_abort_message(const char* msg) {
    fprintf(stderr, "Abort message: %s\n", msg ? msg : "");
}

static int shim_system_property_get(const char* name, char* value) {
    if (value) value[0] = '\0';
    return 0;
}

static void shim_gcov_dump() {}
static void shim_gcov_flush() {}
static int shim_pthread_atfork(void (*prepare)(void), void (*parent)(void), void (*child)(void)) { return 0; }
static void shim_libc_init() {}
static int shim_dummy_trace() { return 0; }

// sysconf translation
static long shim_sysconf(int name) {
    switch (name) {
        case 0x0027: // bionic _SC_PAGESIZE
        case 0x0028: // bionic _SC_PAGE_SIZE
            return sysconf(_SC_PAGESIZE);
        case 0x0060: // bionic _SC_NPROCESSORS_CONF
            return sysconf(_SC_NPROCESSORS_CONF);
        case 0x0061: // bionic _SC_NPROCESSORS_ONLN
            return sysconf(_SC_NPROCESSORS_ONLN);
        case 0x0062: // bionic _SC_PHYS_PAGES
            return sysconf(_SC_PHYS_PAGES);
        case 0x0063: // bionic _SC_AVPHYS_PAGES
            return sysconf(_SC_AVPHYS_PAGES);
        case 0x0006: // bionic _SC_CLK_TCK
            return sysconf(_SC_CLK_TCK);
        case 0x000b: // bionic _SC_OPEN_MAX
            return sysconf(_SC_OPEN_MAX);
        case 0x0005: // bionic _SC_CHILD_MAX
            return sysconf(_SC_CHILD_MAX);
        default:
            return sysconf(name);
    }
}

static int fake_android_log_print(int prio, const char *tag, const char *fmt, ...) {
    va_list ap; va_start(ap, fmt);
    printf("[%s] ", tag ? tag : "android");
    vprintf(fmt, ap);
    printf("\n");
    va_end(ap);
    return 0;
}

static int fake_android_log_write(int prio, const char *tag, const char *text) {
    printf("[%s] %s\n", tag ? tag : "android", text ? text : "");
    return 0;
}

static int fake_android_log_vprint(int prio, const char *tag, const char *fmt, va_list ap) {
    printf("[%s] ", tag ? tag : "android");
    vprintf(fmt, ap);
    printf("\n");
    return 0;
}

static int fake_android_log_buf_write(int bufID, int prio, const char *tag, const char *text) {
    printf("[%s] %s\n", tag ? tag : "android", text ? text : "");
    return 0;
}

static void fake_android_log_assert(const char *cond, const char *tag, const char *fmt, ...) {
    printf("[%s] assert: %s\n", tag ? tag : "android", cond ? cond : "");
}

static void* fake_AAssetManager_fromJava(void* env, void* obj) { return (void*)1; }
static void* fake_AAssetManager_open(void* mgr, const char* filename, int mode) { return nullptr; }
static void fake_AAsset_close(void* asset) {}
static int fake_AAsset_read(void* asset, void* buf, size_t count) { return 0; }
static long fake_AAsset_seek(void* asset, long offset, int whence) { return 0; }
static long fake_AAsset_getLength(void* asset) { return 0; }
static const void* fake_AAsset_getBuffer(void* asset) { return nullptr; }
static int fake_AAsset_openFileDescriptor(void* asset, off_t* outStart, off_t* outLength) { return -1; }

static void* fake_AConfiguration_new() { return (void*)1; }
static void fake_AConfiguration_delete(void*) {}
static void fake_AConfiguration_fromAssetManager(void*, void*) {}
static void fake_AConfiguration_getCountry(void*, char* out) { if (out) { out[0]='U'; out[1]='S'; out[2]='\0'; } }
static void fake_AConfiguration_getLanguage(void*, char* out) { if (out) { out[0]='e'; out[1]='n'; out[2]='\0'; } }
static int32_t fake_AConfiguration_getNavHidden(void*) { return 0; }
static int32_t fake_AConfiguration_getScreenHeightDp(void*) { return 1080; }
static int32_t fake_AConfiguration_getScreenWidthDp(void*) { return 1920; }
static int32_t fake_AConfiguration_getScreenSize(void*) { return 0; }

static void* fake_ALooper_forThread() { return (void*)1; }
static void* fake_ALooper_prepare(int) { return (void*)1; }
static void fake_ALooper_acquire(void*) {}
static void fake_ALooper_release(void*) {}
static int fake_ALooper_pollOnce(int, int*, int*, void**) { return -1; }
static int fake_ALooper_addFd(void*, int, int, int, void*, void*) { return 1; }
static int fake_ALooper_removeFd(void*, int) { return 1; }

static void fake_ANativeWindow_acquire(void*) {}
static void fake_ANativeWindow_release(void*) {}
static void* fake_ANativeWindow_fromSurface(void*, void*) { return (void*)1; }
static int32_t fake_ANativeWindow_getWidth(void*) { return 1920; }
static int32_t fake_ANativeWindow_getHeight(void*) { return 1080; }
static int32_t fake_ANativeWindow_getFormat(void*) { return 1; }
static int32_t fake_ANativeWindow_setBuffersGeometry(void*, int32_t, int32_t, int32_t) { return 0; }
static int32_t fake_ANativeWindow_lock(void*, void*, void*) { return 0; }
static int32_t fake_ANativeWindow_unlockAndPost(void*) { return 0; }

static int fake_AndroidBitmap_getInfo(void*, void*, void*) { return 0; }
static int fake_AndroidBitmap_lockPixels(void*, void*, void**) { return 0; }
static int fake_AndroidBitmap_unlockPixels(void*, void*) { return 0; }

static void* fake_slCreateEngine(void** engine, void* a, void* b, void* c, void* d, void* e) { return nullptr; }
static void* fake_xaCreateEngine(void** engine, void* a, void* b, void* c, void* d, void* e) { return nullptr; }

static const char* fake_AMEDIAFORMAT_KEY = "";

static void* g_h_libc = nullptr;
static void* g_h_libm = nullptr;
static void* g_h_libz = nullptr;
static void* g_h_libegl = nullptr;
static void* g_h_libgles = nullptr;

static void* g_vl_libc = nullptr;
static void* g_vl_libm = nullptr;
static void* g_vl_libz = nullptr;
static void* g_vl_libegl = nullptr;
static void* g_vl_libgles = nullptr;
static void* g_vl_libandroid = nullptr;
static void* g_vl_liblog = nullptr;
static void* g_vl_opensles = nullptr;
static void* g_vl_openmaxal = nullptr;
static void* g_vl_mediandk = nullptr;
static void* g_vl_jnigraphics = nullptr;

static std::unordered_set<std::string> g_resolved_symbols;
static std::mutex g_symbol_mutex;
static std::once_flag g_linker_init_flag;

// helper to extract undefined symbols from an ELF file
static std::unordered_set<std::string> get_undefined_symbols(const char* path) {
    std::unordered_set<std::string> symbols;
    int fd = open(path, O_RDONLY);
    if (fd < 0) return symbols;

    struct stat st;
    if (fstat(fd, &st) != 0) { close(fd); return symbols; }

    void* data = mmap(nullptr, st.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
    if (data == MAP_FAILED) { close(fd); return symbols; }

    ElfW(Ehdr)* ehdr = (ElfW(Ehdr)*)data;
    if (memcmp(ehdr->e_ident, ELFMAG, SELFMAG) == 0 && ehdr->e_ident[EI_CLASS] == ELFCLASS64) {
        ElfW(Shdr)* shdrs = (ElfW(Shdr)*)((uintptr_t)data + ehdr->e_shoff);
        ElfW(Shdr)* symtab = nullptr;
        ElfW(Shdr)* strtab = nullptr;
        for (int i = 0; i < ehdr->e_shnum; i++) {
            if (shdrs[i].sh_type == SHT_DYNSYM) {
                symtab = &shdrs[i];
                strtab = &shdrs[symtab->sh_link];
                break;
            }
        }
        if (symtab && strtab) {
            ElfW(Sym)* syms = (ElfW(Sym)*)((uintptr_t)data + symtab->sh_offset);
            const char* strs = (const char*)((uintptr_t)data + strtab->sh_offset);
            int count = symtab->sh_size / sizeof(ElfW(Sym));
            for (int i = 0; i < count; i++) {
                if (syms[i].st_shndx == SHN_UNDEF && syms[i].st_name != 0) {
                    const char* name = strs + syms[i].st_name;
                    if (name[0] != '\0') {
                        symbols.insert(name);
                    }
                }
            }
        }
    }

    munmap(data, st.st_size);
    close(fd);
    return symbols;
}

// resolve and relocate symbols into virtual libraries
static void resolve_symbols_set(const std::unordered_set<std::string>& symbols) {
    std::lock_guard<std::mutex> lock(g_symbol_mutex);

    std::unordered_map<std::string, void*> new_libc;
    std::unordered_map<std::string, void*> new_libm;
    std::unordered_map<std::string, void*> new_libz;
    std::unordered_map<std::string, void*> new_libegl;
    std::unordered_map<std::string, void*> new_libgles;

    for (const auto& sym : symbols) {
        if (g_resolved_symbols.count(sym)) continue;

        // graphics GLES
        if (sym.rfind("gl", 0) == 0 && g_h_libgles) {
            void* s = dlsym(g_h_libgles, sym.c_str());
            if (s) {
                new_libgles[sym] = s;
                g_resolved_symbols.insert(sym);
                continue;
            }
        }

        // graphics EGL
        if (sym.rfind("egl", 0) == 0 && g_h_libegl) {
            void* s = dlsym(g_h_libegl, sym.c_str());
            if (s) {
                new_libegl[sym] = s;
                g_resolved_symbols.insert(sym);
                continue;
            }
        }

        // compression (zlib)
        if ((sym.rfind("inflate", 0) == 0 || sym.rfind("deflate", 0) == 0 ||
             sym.rfind("crc32", 0) == 0 || sym.rfind("adler32", 0) == 0 ||
             sym.rfind("zlib", 0) == 0) && g_h_libz) {
            void* s = dlsym(g_h_libz, sym.c_str());
            if (s) {
                new_libz[sym] = s;
                g_resolved_symbols.insert(sym);
                continue;
            }
        }

        // math (libm)
        if (g_h_libm) {
            void* s = dlsym(g_h_libm, sym.c_str());
            if (s) {
                new_libm[sym] = s;
                new_libc[sym] = s;
                g_resolved_symbols.insert(sym);
                continue;
            }
        }

        // host libc & RTLD_DEFAULT
        void* s = g_h_libc ? dlsym(g_h_libc, sym.c_str()) : nullptr;
        if (!s) s = dlsym(RTLD_DEFAULT, sym.c_str());
        if (s) {
            new_libc[sym] = s;
            g_resolved_symbols.insert(sym);
            continue;
        }
    }

    if (g_vl_libc && !new_libc.empty()) linker::relocate(g_vl_libc, new_libc);
    if (g_vl_libm && !new_libm.empty()) linker::relocate(g_vl_libm, new_libm);
    if (g_vl_libz && !new_libz.empty()) linker::relocate(g_vl_libz, new_libz);
    if (g_vl_libegl && !new_libegl.empty()) linker::relocate(g_vl_libegl, new_libegl);
    if (g_vl_libgles && !new_libgles.empty()) linker::relocate(g_vl_libgles, new_libgles);
}

static void scan_and_resolve_file(const char* filepath) {
    auto und_syms = get_undefined_symbols(filepath);
    if (!und_syms.empty()) {
        resolve_symbols_set(und_syms);
    }
}

static void scan_and_resolve_dir(const char* dirpath) {
    DIR* dir = opendir(dirpath);
    if (!dir) return;

    struct dirent* ent;
    std::unordered_set<std::string> all_und;
    while ((ent = readdir(dir)) != nullptr) {
        const char* name = ent->d_name;
        size_t len = strlen(name);
        if (len > 3 && strcmp(name + len - 3, ".so") == 0) {
            std::string full_path = std::string(dirpath) + "/" + name;
            auto s = get_undefined_symbols(full_path.c_str());
            all_und.insert(s.begin(), s.end());
        }
    }
    closedir(dir);

    if (!all_und.empty()) {
        resolve_symbols_set(all_und);
    }
}

static void ensure_linker_initialized() {
    std::call_once(g_linker_init_flag, []() {
        linker::init();

        // open host shared libraries
        g_h_libc = dlopen("libc.so.6", RTLD_NOW | RTLD_GLOBAL);
        if (!g_h_libc) g_h_libc = dlopen("libc.so", RTLD_NOW | RTLD_GLOBAL);

        g_h_libm = dlopen("libm.so.6", RTLD_NOW | RTLD_GLOBAL);
        if (!g_h_libm) g_h_libm = dlopen("libm.so", RTLD_NOW | RTLD_GLOBAL);

        g_h_libz = dlopen("libz.so.1", RTLD_NOW | RTLD_GLOBAL);
        if (!g_h_libz) g_h_libz = dlopen("libz.so", RTLD_NOW | RTLD_GLOBAL);

        g_h_libegl = dlopen("libEGL.so.1", RTLD_NOW | RTLD_GLOBAL);
        if (!g_h_libegl) g_h_libegl = dlopen("libEGL.so", RTLD_NOW | RTLD_GLOBAL);

        g_h_libgles = dlopen("libGLESv2.so.2", RTLD_NOW | RTLD_GLOBAL);
        if (!g_h_libgles) g_h_libgles = dlopen("libGLESv2.so", RTLD_NOW | RTLD_GLOBAL);

        // base libc.so symbols
        std::unordered_map<std::string, void*> libc_syms;
        libc_syms["__sF"] = (void*)bionic_sF;
        libc_syms["__stack_chk_guard"] = (void*)&bionic_stack_chk_guard;
        libc_syms["__errno"] = (void*)shim_errno;
        libc_syms["__assert2"] = (void*)shim_assert2;
        libc_syms["android_set_abort_message"] = (void*)shim_set_abort_message;
        libc_syms["__system_property_get"] = (void*)shim_system_property_get;
        libc_syms["sysconf"] = (void*)shim_sysconf;
        libc_syms["fflush"] = (void*)shim_fflush;
        libc_syms["fputs"] = (void*)shim_fputs;
        libc_syms["fwrite"] = (void*)shim_fwrite;
        libc_syms["__fwrite_chk"] = (void*)shim_fwrite_chk;
        libc_syms["__write_chk"] = (void*)shim_write_chk;
        libc_syms["__strlen_chk"] = (void*)shim_strlen_chk;
        libc_syms["__strchr_chk"] = (void*)shim_strchr_chk;
        libc_syms["__strncpy_chk2"] = (void*)shim_strncpy_chk2;
        libc_syms["__umask_chk"] = (void*)shim_umask_chk;
        libc_syms["__FD_CLR_chk"] = (void*)shim_FD_CLR_chk;
        libc_syms["__FD_ISSET_chk"] = (void*)shim_FD_ISSET_chk;
        libc_syms["__FD_SET_chk"] = (void*)shim_FD_SET_chk;
        libc_syms["__sendto_chk"] = (void*)shim_sendto_chk;
        libc_syms["__gnu_strerror_r"] = (void*)shim_gnu_strerror_r;
        libc_syms["__gcov_dump"] = (void*)shim_gcov_dump;
        libc_syms["__gcov_flush"] = (void*)shim_gcov_flush;
        libc_syms["pthread_atfork"] = (void*)shim_pthread_atfork;
        libc_syms["__libc_init"] = (void*)shim_libc_init;
        libc_syms["ZSTD_trace_compress_begin"] = (void*)shim_dummy_trace;
        libc_syms["ZSTD_trace_compress_end"] = (void*)shim_dummy_trace;
        libc_syms["ZSTD_trace_decompress_begin"] = (void*)shim_dummy_trace;
        libc_syms["ZSTD_trace_decompress_end"] = (void*)shim_dummy_trace;

        for (const auto& kv : libc_syms) {
            g_resolved_symbols.insert(kv.first);
        }

        // base liblog.so symbols
        std::unordered_map<std::string, void*> log_syms = {
            {"__android_log_print", (void*)fake_android_log_print},
            {"__android_log_write", (void*)fake_android_log_write},
            {"__android_log_vprint", (void*)fake_android_log_vprint},
            {"__android_log_buf_write", (void*)fake_android_log_buf_write},
            {"__android_log_assert", (void*)fake_android_log_assert},
        };
        for (const auto& kv : log_syms) g_resolved_symbols.insert(kv.first);

        // base libandroid.so symbols
        std::unordered_map<std::string, void*> android_syms = {
            {"AAssetManager_fromJava", (void*)fake_AAssetManager_fromJava},
            {"AAssetManager_open", (void*)fake_AAssetManager_open},
            {"AAsset_close", (void*)fake_AAsset_close},
            {"AAsset_read", (void*)fake_AAsset_read},
            {"AAsset_seek", (void*)fake_AAsset_seek},
            {"AAsset_getLength", (void*)fake_AAsset_getLength},
            {"AAsset_getBuffer", (void*)fake_AAsset_getBuffer},
            {"AAsset_openFileDescriptor", (void*)fake_AAsset_openFileDescriptor},
            {"AConfiguration_new", (void*)fake_AConfiguration_new},
            {"AConfiguration_delete", (void*)fake_AConfiguration_delete},
            {"AConfiguration_fromAssetManager", (void*)fake_AConfiguration_fromAssetManager},
            {"AConfiguration_getCountry", (void*)fake_AConfiguration_getCountry},
            {"AConfiguration_getLanguage", (void*)fake_AConfiguration_getLanguage},
            {"AConfiguration_getNavHidden", (void*)fake_AConfiguration_getNavHidden},
            {"AConfiguration_getScreenHeightDp", (void*)fake_AConfiguration_getScreenHeightDp},
            {"AConfiguration_getScreenWidthDp", (void*)fake_AConfiguration_getScreenWidthDp},
            {"AConfiguration_getScreenSize", (void*)fake_AConfiguration_getScreenSize},
            {"ALooper_forThread", (void*)fake_ALooper_forThread},
            {"ALooper_prepare", (void*)fake_ALooper_prepare},
            {"ALooper_acquire", (void*)fake_ALooper_acquire},
            {"ALooper_release", (void*)fake_ALooper_release},
            {"ALooper_pollOnce", (void*)fake_ALooper_pollOnce},
            {"ALooper_addFd", (void*)fake_ALooper_addFd},
            {"ALooper_removeFd", (void*)fake_ALooper_removeFd},
            {"ANativeWindow_acquire", (void*)fake_ANativeWindow_acquire},
            {"ANativeWindow_release", (void*)fake_ANativeWindow_release},
            {"ANativeWindow_fromSurface", (void*)fake_ANativeWindow_fromSurface},
            {"ANativeWindow_getWidth", (void*)fake_ANativeWindow_getWidth},
            {"ANativeWindow_getHeight", (void*)fake_ANativeWindow_getHeight},
            {"ANativeWindow_getFormat", (void*)fake_ANativeWindow_getFormat},
            {"ANativeWindow_setBuffersGeometry", (void*)fake_ANativeWindow_setBuffersGeometry},
            {"ANativeWindow_lock", (void*)fake_ANativeWindow_lock},
            {"ANativeWindow_unlockAndPost", (void*)fake_ANativeWindow_unlockAndPost},
        };
        for (const auto& kv : android_syms) g_resolved_symbols.insert(kv.first);

        // base jnigraphics
        std::unordered_map<std::string, void*> jnigraphics_syms = {
            {"AndroidBitmap_getInfo", (void*)fake_AndroidBitmap_getInfo},
            {"AndroidBitmap_lockPixels", (void*)fake_AndroidBitmap_lockPixels},
            {"AndroidBitmap_unlockPixels", (void*)fake_AndroidBitmap_unlockPixels},
        };
        for (const auto& kv : jnigraphics_syms) g_resolved_symbols.insert(kv.first);

        // OpenSLES / OpenMAXAL
        std::unordered_map<std::string, void*> opensles_syms = {
            {"slCreateEngine", (void*)fake_slCreateEngine},
        };
        for (const auto& kv : opensles_syms) g_resolved_symbols.insert(kv.first);

        std::unordered_map<std::string, void*> openmaxal_syms = {
            {"xaCreateEngine", (void*)fake_xaCreateEngine},
        };
        for (const auto& kv : openmaxal_syms) g_resolved_symbols.insert(kv.first);

        // MediaNDK
        std::unordered_map<std::string, void*> mediandk_syms;
        const char* media_format_keys[] = {
            "AMEDIAFORMAT_KEY_BIT_RATE", "AMEDIAFORMAT_KEY_CHANNEL_COUNT", "AMEDIAFORMAT_KEY_COLOR_FORMAT",
            "AMEDIAFORMAT_KEY_FRAME_RATE", "AMEDIAFORMAT_KEY_HEIGHT", "AMEDIAFORMAT_KEY_I_FRAME_INTERVAL",
            "AMEDIAFORMAT_KEY_MIME", "AMEDIAFORMAT_KEY_SAMPLE_RATE", "AMEDIAFORMAT_KEY_STRIDE", "AMEDIAFORMAT_KEY_WIDTH"
        };
        for (auto k : media_format_keys) {
            mediandk_syms[k] = (void*)&fake_AMEDIAFORMAT_KEY;
            g_resolved_symbols.insert(k);
        }
        const char* media_funcs[] = {
            "AMediaCodec_configure", "AMediaCodec_createDecoderByType", "AMediaCodec_createEncoderByType",
            "AMediaCodec_delete", "AMediaCodec_dequeueInputBuffer", "AMediaCodec_dequeueOutputBuffer",
            "AMediaCodec_flush", "AMediaCodec_getInputBuffer", "AMediaCodec_getOutputBuffer",
            "AMediaCodec_getOutputFormat", "AMediaCodec_queueInputBuffer", "AMediaCodec_releaseOutputBuffer",
            "AMediaCodec_start", "AMediaCodec_stop", "AMediaFormat_delete", "AMediaFormat_getBuffer",
            "AMediaFormat_getInt32", "AMediaFormat_new", "AMediaFormat_setBuffer", "AMediaFormat_setFloat",
            "AMediaFormat_setInt32", "AMediaFormat_setString", "AMediaFormat_toString"
        };
        for (auto f : media_funcs) {
            mediandk_syms[f] = (void*)shim_dummy_trace;
            g_resolved_symbols.insert(f);
        }

        // empty initial maps for dynamic libs
        std::unordered_map<std::string, void*> empty_syms;

        // register all in-memory virtual libraries
        g_vl_libc = linker::load_library("libc.so", libc_syms);
        g_vl_libm = linker::load_library("libm.so", empty_syms);
        g_vl_libz = linker::load_library("libz.so", empty_syms);
        g_vl_libegl = linker::load_library("libEGL.so", empty_syms);
        g_vl_libgles = linker::load_library("libGLESv2.so", empty_syms);
        g_vl_libandroid = linker::load_library("libandroid.so", android_syms);
        g_vl_liblog = linker::load_library("liblog.so", log_syms);
        g_vl_opensles = linker::load_library("libOpenSLES.so", opensles_syms);
        g_vl_openmaxal = linker::load_library("libOpenMAXAL.so", openmaxal_syms);
        g_vl_mediandk = linker::load_library("libmediandk.so", mediandk_syms);
        g_vl_jnigraphics = linker::load_library("libjnigraphics.so", jnigraphics_syms);
    });
}

extern "C" {

void* cork_linker_load_library(const char* path) {
    if (path == nullptr) return nullptr;
    ensure_linker_initialized();

    // resolve any undefined symbols from the binary before dlopen
    scan_and_resolve_file(path);

    return linker::dlopen(path, 2);
}

void* cork_linker_get_symbol(void* handle, const char* symbol_name) {
    if (handle == nullptr || symbol_name == nullptr) return nullptr;
    return linker::dlsym(handle, symbol_name);
}

const char* cork_linker_get_error(void) {
    return __loader_dlerror();
}

void cork_linker_set_search_path(const char* path) {
    if (path != nullptr) {
        ensure_linker_initialized();
        __loader_android_update_LD_LIBRARY_PATH(path);

        // scan all directories in LD_LIBRARY_PATH for .so files
        std::string p(path);
        size_t start = 0;
        while (start < p.size()) {
            size_t end = p.find(':', start);
            if (end == std::string::npos) end = p.size();
            std::string dir = p.substr(start, end - start);
            if (!dir.empty()) {
                scan_and_resolve_dir(dir.c_str());
            }
            start = end + 1;
        }
    }
}

int cork_linker_call_jni_onload(void* handle, void* java_vm) {
    if (!handle || !java_vm) return 0;
    typedef int (*JniOnLoadFn)(void* vm, void* reserved);
    JniOnLoadFn on_load = (JniOnLoadFn)linker::dlsym(handle, "JNI_OnLoad");
    if (!on_load) return 0;

    return on_load(java_vm, nullptr);
}

}