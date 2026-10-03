#pragma once

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
