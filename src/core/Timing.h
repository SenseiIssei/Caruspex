// Kleine Zeit-Helfer, damit der Kern ohne Betriebssystem-Header auskommt.
#pragma once

#include <chrono>
#include <cstdint>
#include <thread>

namespace caruspex {

// Millisekunden seit einem beliebigen Startpunkt, monoton steigend.
inline std::uint64_t nowMs() {
    using namespace std::chrono;
    return static_cast<std::uint64_t>(
        duration_cast<milliseconds>(steady_clock::now().time_since_epoch()).count());
}

inline void sleepMs(int milliseconds) {
    std::this_thread::sleep_for(std::chrono::milliseconds(milliseconds));
}

}  // namespace caruspex
