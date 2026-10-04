use std::ffi::{VaList, c_char, c_int, c_long, c_void};

static mut BIONIC_SF: [[u8; 152]; 3] = [[0; 152]; 3];
static BIONIC_STACK_CHK_GUARD: usize = 0x595e_9fbd_94fd_a766;
static FAKE_AMEDIAFORMAT_KEY: [u8; 1] = [0];

unsafe extern "C" {
    static mut stdin: *mut libc::FILE;
    static mut stdout: *mut libc::FILE;
    static mut stderr: *mut libc::FILE;
    fn vprintf(fmt: *const c_char, ap: VaList) -> c_int;
    fn printf(fmt: *const c_char, _: ...) -> c_int;
    fn putchar(c: c_int) -> c_int;
}

#[must_use]
pub unsafe fn unwrap_file(s: *mut c_void) -> *mut libc::FILE {
    let p = s as usize;
    let b0 = std::ptr::addr_of!(BIONIC_SF) as usize;
    let b1 = b0 + 152;
    let b2 = b0 + 304;
    if p == b0 {
        unsafe { stdin }
    } else if p == b1 {
        unsafe { stdout }
    } else if p == b2 {
        unsafe { stderr }
    } else {
        s.cast()
    }
}

pub unsafe extern "C" fn shim_fflush(stream: *mut c_void) -> c_int {
    if stream.is_null() {
        unsafe { libc::fflush(std::ptr::null_mut()) }
    } else {
        unsafe { libc::fflush(unwrap_file(stream)) }
    }
}

pub unsafe extern "C" fn shim_fputs(s: *const c_char, stream: *mut c_void) -> c_int {
    unsafe { libc::fputs(s, unwrap_file(stream)) }
}

pub unsafe extern "C" fn shim_fwrite(
    ptr: *const c_void,
    size: usize,
    nmemb: usize,
    stream: *mut c_void,
) -> usize {
    unsafe { libc::fwrite(ptr, size, nmemb, unwrap_file(stream)) }
}

pub unsafe extern "C" fn shim_fwrite_chk(
    ptr: *const c_void,
    size: usize,
    nmemb: usize,
    stream: *mut c_void,
    _chk: usize,
) -> usize {
    unsafe { libc::fwrite(ptr, size, nmemb, unwrap_file(stream)) }
}

pub unsafe extern "C" fn shim_write_chk(
    fd: c_int,
    buf: *const c_void,
    count: usize,
    _chk: usize,
) -> isize {
    unsafe { libc::write(fd, buf, count) }
}

pub unsafe extern "C" fn shim_strlen_chk(s: *const c_char, _chk: usize) -> usize {
    unsafe { libc::strlen(s) }
}

pub unsafe extern "C" fn shim_strchr_chk(s: *const c_char, c: c_int, _chk: usize) -> *mut c_char {
    unsafe { libc::strchr(s, c) }
}

pub unsafe extern "C" fn shim_strncpy_chk2(
    dst: *mut c_char,
    src: *const c_char,
    n: usize,
    _chk1: usize,
    _chk2: usize,
) -> *mut c_char {
    unsafe { libc::strncpy(dst, src, n) }
}

pub unsafe extern "C" fn shim_umask_chk(mask: libc::mode_t) -> libc::mode_t {
    unsafe { libc::umask(mask) }
}

pub unsafe extern "C" fn shim_fd_clr_chk(fd: c_int, set: *mut libc::fd_set, _chk: usize) {
    unsafe { libc::FD_CLR(fd, set) };
}

pub unsafe extern "C" fn shim_fd_isset_chk(
    fd: c_int,
    set: *mut libc::fd_set,
    _chk: usize,
) -> c_int {
    i32::from(unsafe { libc::FD_ISSET(fd, set) })
}

pub unsafe extern "C" fn shim_fd_set_chk(fd: c_int, set: *mut libc::fd_set, _chk: usize) {
    unsafe { libc::FD_SET(fd, set) };
}

pub unsafe extern "C" fn shim_sendto_chk(
    s: c_int,
    buf: *const c_void,
    len: usize,
    _chk: usize,
    flags: c_int,
    to: *const libc::sockaddr,
    tolen: libc::socklen_t,
) -> isize {
    unsafe { libc::sendto(s, buf, len, flags, to, tolen) }
}

pub unsafe extern "C" fn shim_gnu_strerror_r(
    errnum: c_int,
    buf: *mut c_char,
    buflen: usize,
) -> *mut c_char {
    unsafe { libc::strerror_r(errnum, buf, buflen) };
    buf
}

pub unsafe extern "C" fn shim_errno() -> *mut c_int {
    unsafe { libc::__errno_location() }
}

pub unsafe extern "C" fn shim_assert2(
    file: *const c_char,
    line: c_int,
    func: *const c_char,
    expr: *const c_char,
) {
    let f = if file.is_null() { c"".as_ptr() } else { file };
    let fnc = if func.is_null() { c"".as_ptr() } else { func };
    let e = if expr.is_null() { c"".as_ptr() } else { expr };
    unsafe {
        libc::fprintf(
            stderr,
            c"Assertion failed: %s (%s:%d: %s)\n".as_ptr(),
            e,
            f,
            line,
            fnc,
        );
        libc::abort();
    }
}

pub unsafe extern "C" fn shim_set_abort_message(msg: *const c_char) {
    let m = if msg.is_null() { c"".as_ptr() } else { msg };
    unsafe {
        libc::fprintf(stderr, c"Abort message: %s\n".as_ptr(), m);
    }
}

pub unsafe extern "C" fn shim_system_property_get(
    _name: *const c_char,
    value: *mut c_char,
) -> c_int {
    if !value.is_null() {
        unsafe { *value = 0 };
    }
    0
}

pub unsafe extern "C" fn shim_sysconf(name: c_int) -> c_long {
    match name {
        0x0027 | 0x0028 => unsafe { libc::sysconf(libc::_SC_PAGESIZE) },
        0x0060 => unsafe { libc::sysconf(libc::_SC_NPROCESSORS_CONF) },
        0x0061 => unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) },
        0x0062 => unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) },
        0x0063 => unsafe { libc::sysconf(libc::_SC_AVPHYS_PAGES) },
        0x0006 => unsafe { libc::sysconf(libc::_SC_CLK_TCK) },
        0x000b => unsafe { libc::sysconf(libc::_SC_OPEN_MAX) },
        0x0005 => unsafe { libc::sysconf(libc::_SC_CHILD_MAX) },
        _ => unsafe { libc::sysconf(name) },
    }
}

pub unsafe extern "C" fn shim_gcov_dump() {}
pub unsafe extern "C" fn shim_gcov_flush() {}
pub unsafe extern "C" fn shim_pthread_atfork(
    _prepare: Option<unsafe extern "C" fn()>,
    _parent: Option<unsafe extern "C" fn()>,
    _child: Option<unsafe extern "C" fn()>,
) -> c_int {
    0
}
pub unsafe extern "C" fn shim_libc_init() {}
pub unsafe extern "C" fn shim_dummy_trace() -> c_int {
    0
}

// android log
pub unsafe extern "C" fn fake_android_log_print(
    _prio: c_int,
    tag: *const c_char,
    fmt: *const c_char,
    args: ...
) -> c_int {
    let tag_str = if tag.is_null() {
        c"android".as_ptr()
    } else {
        tag
    };
    unsafe {
        printf(c"[%s] ".as_ptr(), tag_str);
        vprintf(fmt, args);
        putchar(10);
    }
    0
}

pub unsafe extern "C" fn fake_android_log_write(
    _prio: c_int,
    tag: *const c_char,
    text: *const c_char,
) -> c_int {
    let tag_str = if tag.is_null() {
        c"android".as_ptr()
    } else {
        tag
    };
    let text_str = if text.is_null() { c"".as_ptr() } else { text };
    unsafe {
        printf(c"[%s] %s\n".as_ptr(), tag_str, text_str);
    }
    0
}

pub unsafe extern "C" fn fake_android_log_vprint(
    _prio: c_int,
    tag: *const c_char,
    fmt: *const c_char,
    ap: VaList,
) -> c_int {
    let tag_str = if tag.is_null() {
        c"android".as_ptr()
    } else {
        tag
    };
    unsafe {
        printf(c"[%s] ".as_ptr(), tag_str);
        vprintf(fmt, ap);
        putchar(10);
    }
    0
}

pub unsafe extern "C" fn fake_android_log_buf_write(
    _buf_id: c_int,
    prio: c_int,
    tag: *const c_char,
    text: *const c_char,
) -> c_int {
    unsafe { fake_android_log_write(prio, tag, text) }
}

pub unsafe extern "C" fn fake_android_log_assert(
    cond: *const c_char,
    tag: *const c_char,
    _fmt: *const c_char,
    _: ...
) {
    let tag_str = if tag.is_null() {
        c"android".as_ptr()
    } else {
        tag
    };
    let cond_str = if cond.is_null() { c"".as_ptr() } else { cond };
    unsafe {
        printf(c"[%s] assert: %s\n".as_ptr(), tag_str, cond_str);
    }
}

// AAssetManager & AAsset
pub unsafe extern "C" fn fake_aasset_manager_from_java(
    _env: *mut c_void,
    _obj: *mut c_void,
) -> *mut c_void {
    std::ptr::without_provenance_mut(1)
}
pub unsafe extern "C" fn fake_aasset_manager_open(
    _mgr: *mut c_void,
    _filename: *const c_char,
    _mode: c_int,
) -> *mut c_void {
    std::ptr::null_mut()
}
pub unsafe extern "C" fn fake_aasset_close(_asset: *mut c_void) {}
pub unsafe extern "C" fn fake_aasset_read(
    _asset: *mut c_void,
    _buf: *mut c_void,
    _count: usize,
) -> c_int {
    0
}
pub unsafe extern "C" fn fake_aasset_seek(
    _asset: *mut c_void,
    _offset: c_long,
    _whence: c_int,
) -> c_long {
    0
}
pub unsafe extern "C" fn fake_aasset_get_length(_asset: *mut c_void) -> c_long {
    0
}
pub unsafe extern "C" fn fake_aasset_get_buffer(_asset: *mut c_void) -> *const c_void {
    std::ptr::null()
}
pub unsafe extern "C" fn fake_aasset_open_file_descriptor(
    _asset: *mut c_void,
    _out_start: *mut libc::off_t,
    _out_len: *mut libc::off_t,
) -> c_int {
    -1
}

// AConfiguration
pub unsafe extern "C" fn fake_aconfiguration_new() -> *mut c_void {
    std::ptr::without_provenance_mut(1)
}
pub unsafe extern "C" fn fake_aconfiguration_delete(_cfg: *mut c_void) {}
pub unsafe extern "C" fn fake_aconfiguration_from_asset_manager(
    _cfg: *mut c_void,
    _mgr: *mut c_void,
) {
}
pub unsafe extern "C" fn fake_aconfiguration_get_country(_cfg: *mut c_void, out: *mut c_char) {
    if !out.is_null() {
        unsafe {
            *out = b'U' as c_char;
            *out.add(1) = b'S' as c_char;
            *out.add(2) = 0;
        }
    }
}
pub unsafe extern "C" fn fake_aconfiguration_get_language(_cfg: *mut c_void, out: *mut c_char) {
    if !out.is_null() {
        unsafe {
            *out = b'e' as c_char;
            *out.add(1) = b'n' as c_char;
            *out.add(2) = 0;
        }
    }
}
pub unsafe extern "C" fn fake_aconfiguration_get_nav_hidden(_cfg: *mut c_void) -> i32 {
    0
}
pub unsafe extern "C" fn fake_aconfiguration_get_screen_height_dp(_cfg: *mut c_void) -> i32 {
    1080
}
pub unsafe extern "C" fn fake_aconfiguration_get_screen_width_dp(_cfg: *mut c_void) -> i32 {
    1920
}
pub unsafe extern "C" fn fake_aconfiguration_get_screen_size(_cfg: *mut c_void) -> i32 {
    0
}

// ALooper
pub unsafe extern "C" fn fake_alooper_for_thread() -> *mut c_void {
    std::ptr::without_provenance_mut(1)
}
pub unsafe extern "C" fn fake_alooper_prepare(_opts: c_int) -> *mut c_void {
    std::ptr::without_provenance_mut(1)
}
pub unsafe extern "C" fn fake_alooper_acquire(_looper: *mut c_void) {}
pub unsafe extern "C" fn fake_alooper_release(_looper: *mut c_void) {}
pub unsafe extern "C" fn fake_alooper_poll_once(
    _timeout_ms: c_int,
    _out_fd: *mut c_int,
    _out_events: *mut c_int,
    _out_data: *mut *mut c_void,
) -> c_int {
    -1
}
pub unsafe extern "C" fn fake_alooper_add_fd(
    _looper: *mut c_void,
    _fd: c_int,
    _ident: c_int,
    _events: c_int,
    _callback: *mut c_void,
    _data: *mut c_void,
) -> c_int {
    1
}
pub unsafe extern "C" fn fake_alooper_remove_fd(_looper: *mut c_void, _fd: c_int) -> c_int {
    1
}

// ANativeWindow
pub unsafe extern "C" fn fake_anative_window_acquire(_w: *mut c_void) {}
pub unsafe extern "C" fn fake_anative_window_release(_w: *mut c_void) {}
pub unsafe extern "C" fn fake_anative_window_from_surface(
    _env: *mut c_void,
    _surface: *mut c_void,
) -> *mut c_void {
    std::ptr::without_provenance_mut(1)
}
pub unsafe extern "C" fn fake_anative_window_get_width(_w: *mut c_void) -> i32 {
    1920
}
pub unsafe extern "C" fn fake_anative_window_get_height(_w: *mut c_void) -> i32 {
    1080
}
pub unsafe extern "C" fn fake_anative_window_get_format(_w: *mut c_void) -> i32 {
    1
}
pub unsafe extern "C" fn fake_anative_window_set_buffers_geometry(
    _w: *mut c_void,
    _width: i32,
    _height: i32,
    _format: i32,
) -> i32 {
    0
}
pub unsafe extern "C" fn fake_anative_window_lock(
    _w: *mut c_void,
    _out_buf: *mut c_void,
    _bounds: *mut c_void,
) -> i32 {
    0
}
pub unsafe extern "C" fn fake_anative_window_unlock_and_post(_w: *mut c_void) -> i32 {
    0
}

// AndroidBitmap
pub unsafe extern "C" fn fake_android_bitmap_get_info(
    _env: *mut c_void,
    _bm: *mut c_void,
    _info: *mut c_void,
) -> c_int {
    0
}
pub unsafe extern "C" fn fake_android_bitmap_lock_pixels(
    _env: *mut c_void,
    _bm: *mut c_void,
    _pixels: *mut *mut c_void,
) -> c_int {
    0
}
pub unsafe extern "C" fn fake_android_bitmap_unlock_pixels(
    _env: *mut c_void,
    _bm: *mut c_void,
) -> c_int {
    0
}

// OpenSLES / OpenMAXAL
pub unsafe extern "C" fn fake_sl_create_engine(
    _engine: *mut *mut c_void,
    _: *mut c_void,
    _: *mut c_void,
    _: *mut c_void,
    _: *mut c_void,
    _: *mut c_void,
) -> *mut c_void {
    std::ptr::null_mut()
}
pub unsafe extern "C" fn fake_xa_create_engine(
    _engine: *mut *mut c_void,
    _: *mut c_void,
    _: *mut c_void,
    _: *mut c_void,
    _: *mut c_void,
    _: *mut c_void,
) -> *mut c_void {
    std::ptr::null_mut()
}

#[must_use]
pub fn get_libc_shims() -> Vec<(&'static str, *mut c_void)> {
    vec![
        ("__sF", std::ptr::addr_of_mut!(BIONIC_SF).cast()),
        (
            "__stack_chk_guard",
            std::ptr::addr_of!(BIONIC_STACK_CHK_GUARD) as *mut c_void,
        ),
        ("__errno", shim_errno as *mut c_void),
        ("__assert2", shim_assert2 as *mut c_void),
        (
            "android_set_abort_message",
            shim_set_abort_message as *mut c_void,
        ),
        (
            "__system_property_get",
            shim_system_property_get as *mut c_void,
        ),
        ("sysconf", shim_sysconf as *mut c_void),
        ("fflush", shim_fflush as *mut c_void),
        ("fputs", shim_fputs as *mut c_void),
        ("fwrite", shim_fwrite as *mut c_void),
        ("__fwrite_chk", shim_fwrite_chk as *mut c_void),
        ("__write_chk", shim_write_chk as *mut c_void),
        ("__strlen_chk", shim_strlen_chk as *mut c_void),
        ("__strchr_chk", shim_strchr_chk as *mut c_void),
        ("__strncpy_chk2", shim_strncpy_chk2 as *mut c_void),
        ("__umask_chk", shim_umask_chk as *mut c_void),
        ("__FD_CLR_chk", shim_fd_clr_chk as *mut c_void),
        ("__FD_ISSET_chk", shim_fd_isset_chk as *mut c_void),
        ("__FD_SET_chk", shim_fd_set_chk as *mut c_void),
        ("__sendto_chk", shim_sendto_chk as *mut c_void),
        ("__gnu_strerror_r", shim_gnu_strerror_r as *mut c_void),
        ("__gcov_dump", shim_gcov_dump as *mut c_void),
        ("__gcov_flush", shim_gcov_flush as *mut c_void),
        ("pthread_atfork", shim_pthread_atfork as *mut c_void),
        ("__libc_init", shim_libc_init as *mut c_void),
        ("ZSTD_trace_compress_begin", shim_dummy_trace as *mut c_void),
        ("ZSTD_trace_compress_end", shim_dummy_trace as *mut c_void),
        (
            "ZSTD_trace_decompress_begin",
            shim_dummy_trace as *mut c_void,
        ),
        ("ZSTD_trace_decompress_end", shim_dummy_trace as *mut c_void),
    ]
}

#[must_use]
pub fn get_liblog_shims() -> Vec<(&'static str, *mut c_void)> {
    vec![
        ("__android_log_print", fake_android_log_print as *mut c_void),
        ("__android_log_write", fake_android_log_write as *mut c_void),
        (
            "__android_log_vprint",
            fake_android_log_vprint as *mut c_void,
        ),
        (
            "__android_log_buf_write",
            fake_android_log_buf_write as *mut c_void,
        ),
        (
            "__android_log_assert",
            fake_android_log_assert as *mut c_void,
        ),
    ]
}

#[must_use]
pub fn get_libandroid_shims() -> Vec<(&'static str, *mut c_void)> {
    vec![
        (
            "AAssetManager_fromJava",
            fake_aasset_manager_from_java as *mut c_void,
        ),
        (
            "AAssetManager_open",
            fake_aasset_manager_open as *mut c_void,
        ),
        ("AAsset_close", fake_aasset_close as *mut c_void),
        ("AAsset_read", fake_aasset_read as *mut c_void),
        ("AAsset_seek", fake_aasset_seek as *mut c_void),
        ("AAsset_getLength", fake_aasset_get_length as *mut c_void),
        ("AAsset_getBuffer", fake_aasset_get_buffer as *mut c_void),
        (
            "AAsset_openFileDescriptor",
            fake_aasset_open_file_descriptor as *mut c_void,
        ),
        ("AConfiguration_new", fake_aconfiguration_new as *mut c_void),
        (
            "AConfiguration_delete",
            fake_aconfiguration_delete as *mut c_void,
        ),
        (
            "AConfiguration_fromAssetManager",
            fake_aconfiguration_from_asset_manager as *mut c_void,
        ),
        (
            "AConfiguration_getCountry",
            fake_aconfiguration_get_country as *mut c_void,
        ),
        (
            "AConfiguration_getLanguage",
            fake_aconfiguration_get_language as *mut c_void,
        ),
        (
            "AConfiguration_getNavHidden",
            fake_aconfiguration_get_nav_hidden as *mut c_void,
        ),
        (
            "AConfiguration_getScreenHeightDp",
            fake_aconfiguration_get_screen_height_dp as *mut c_void,
        ),
        (
            "AConfiguration_getScreenWidthDp",
            fake_aconfiguration_get_screen_width_dp as *mut c_void,
        ),
        (
            "AConfiguration_getScreenSize",
            fake_aconfiguration_get_screen_size as *mut c_void,
        ),
        ("ALooper_forThread", fake_alooper_for_thread as *mut c_void),
        ("ALooper_prepare", fake_alooper_prepare as *mut c_void),
        ("ALooper_acquire", fake_alooper_acquire as *mut c_void),
        ("ALooper_release", fake_alooper_release as *mut c_void),
        ("ALooper_pollOnce", fake_alooper_poll_once as *mut c_void),
        ("ALooper_addFd", fake_alooper_add_fd as *mut c_void),
        ("ALooper_removeFd", fake_alooper_remove_fd as *mut c_void),
        (
            "ANativeWindow_acquire",
            fake_anative_window_acquire as *mut c_void,
        ),
        (
            "ANativeWindow_release",
            fake_anative_window_release as *mut c_void,
        ),
        (
            "ANativeWindow_fromSurface",
            fake_anative_window_from_surface as *mut c_void,
        ),
        (
            "ANativeWindow_getWidth",
            fake_anative_window_get_width as *mut c_void,
        ),
        (
            "ANativeWindow_getHeight",
            fake_anative_window_get_height as *mut c_void,
        ),
        (
            "ANativeWindow_getFormat",
            fake_anative_window_get_format as *mut c_void,
        ),
        (
            "ANativeWindow_setBuffersGeometry",
            fake_anative_window_set_buffers_geometry as *mut c_void,
        ),
        (
            "ANativeWindow_lock",
            fake_anative_window_lock as *mut c_void,
        ),
        (
            "ANativeWindow_unlockAndPost",
            fake_anative_window_unlock_and_post as *mut c_void,
        ),
    ]
}

#[must_use]
pub fn get_jnigraphics_shims() -> Vec<(&'static str, *mut c_void)> {
    vec![
        (
            "AndroidBitmap_getInfo",
            fake_android_bitmap_get_info as *mut c_void,
        ),
        (
            "AndroidBitmap_lockPixels",
            fake_android_bitmap_lock_pixels as *mut c_void,
        ),
        (
            "AndroidBitmap_unlockPixels",
            fake_android_bitmap_unlock_pixels as *mut c_void,
        ),
    ]
}

#[must_use]
pub fn get_opensles_shims() -> Vec<(&'static str, *mut c_void)> {
    vec![("slCreateEngine", fake_sl_create_engine as *mut c_void)]
}

#[must_use]
pub fn get_openmaxal_shims() -> Vec<(&'static str, *mut c_void)> {
    vec![("xaCreateEngine", fake_xa_create_engine as *mut c_void)]
}

#[must_use]
pub fn get_mediandk_shims() -> Vec<(&'static str, *mut c_void)> {
    let dummy_key = std::ptr::addr_of!(FAKE_AMEDIAFORMAT_KEY) as *mut c_void;
    let dummy_fn = shim_dummy_trace as *mut c_void;

    vec![
        ("AMEDIAFORMAT_KEY_BIT_RATE", dummy_key),
        ("AMEDIAFORMAT_KEY_CHANNEL_COUNT", dummy_key),
        ("AMEDIAFORMAT_KEY_COLOR_FORMAT", dummy_key),
        ("AMEDIAFORMAT_KEY_FRAME_RATE", dummy_key),
        ("AMEDIAFORMAT_KEY_HEIGHT", dummy_key),
        ("AMEDIAFORMAT_KEY_I_FRAME_INTERVAL", dummy_key),
        ("AMEDIAFORMAT_KEY_MIME", dummy_key),
        ("AMEDIAFORMAT_KEY_SAMPLE_RATE", dummy_key),
        ("AMEDIAFORMAT_KEY_STRIDE", dummy_key),
        ("AMEDIAFORMAT_KEY_WIDTH", dummy_key),
        ("AMediaCodec_configure", dummy_fn),
        ("AMediaCodec_createDecoderByType", dummy_fn),
        ("AMediaCodec_createEncoderByType", dummy_fn),
        ("AMediaCodec_delete", dummy_fn),
        ("AMediaCodec_dequeueInputBuffer", dummy_fn),
        ("AMediaCodec_dequeueOutputBuffer", dummy_fn),
        ("AMediaCodec_flush", dummy_fn),
        ("AMediaCodec_getInputBuffer", dummy_fn),
        ("AMediaCodec_getOutputBuffer", dummy_fn),
        ("AMediaCodec_getOutputFormat", dummy_fn),
        ("AMediaCodec_queueInputBuffer", dummy_fn),
        ("AMediaCodec_releaseOutputBuffer", dummy_fn),
        ("AMediaCodec_start", dummy_fn),
        ("AMediaCodec_stop", dummy_fn),
        ("AMediaFormat_delete", dummy_fn),
        ("AMediaFormat_getBuffer", dummy_fn),
        ("AMediaFormat_getInt32", dummy_fn),
        ("AMediaFormat_new", dummy_fn),
        ("AMediaFormat_setBuffer", dummy_fn),
        ("AMediaFormat_setFloat", dummy_fn),
        ("AMediaFormat_setInt32", dummy_fn),
        ("AMediaFormat_setString", dummy_fn),
        ("AMediaFormat_toString", dummy_fn),
    ]
}
