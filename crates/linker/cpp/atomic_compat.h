#pragma once

#ifndef __has_feature
#define __has_feature(x) 0
#endif

#ifndef __has_extension
#define __has_extension(x) 0
#endif

#ifndef __has_attribute
#define __has_attribute(x) 0
#endif

#ifndef __has_builtin
#define __has_builtin(x) 0
#endif

#if defined(__cplusplus) && !defined(__clang__)
#include <atomic>
#include <cstdint>
#include <cstddef>

#ifndef _Atomic
#define _Atomic(T) std::atomic<T>
#endif

using std::atomic_int;
using std::atomic_bool;
using std::atomic_uint;
#endif
