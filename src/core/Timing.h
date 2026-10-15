// Small timing helpers so the core gets by without operating system headers.
#pragma once

#include <chrono>
#include <cstdint>
#include <thread>

namespace caruspex {

// Milliseconds since an arbitrary starting point, monotonically increasing.
inline std::uint64_t nowMs() {
    using namespace std::chrono;
    return static_cast<std::uint64_t>(
        duration_cast<milliseconds>(steady_clock::now().time_since_epoch()).count());
}

inline void sleepMs(int milliseconds) {
    std::this_thread::sleep_for(std::chrono::milliseconds(milliseconds));
}

// Waits until a fixed point in time instead of for a duration.
//
// Operating systems like to sleep a few milliseconds too long. The 5-baud
// wake-up address strings ten bits together, so the overshoot adds up. With a
// fixed target per bit the error stays the same for every bit instead of
// accumulating.
inline void sleepUntilMs(std::uint64_t deadline) {
    for (;;) {
        const std::uint64_t now = nowMs();
        if (now >= deadline) return;
        const std::uint64_t remaining = deadline - now;
        // Busy-wait for the last bit, otherwise we oversleep the deadline.
        if (remaining > 3) {
            std::this_thread::sleep_for(std::chrono::milliseconds(remaining - 2));
        } else {
            std::this_thread::yield();
        }
    }
}

}  // namespace caruspex
