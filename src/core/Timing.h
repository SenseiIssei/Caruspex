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

// Wartet bis zu einem festen Zeitpunkt statt eine Dauer lang.
//
// Betriebssysteme schlafen gern ein paar Millisekunden zu lang. Bei der
// 5-Baud-Weckadresse haengen zehn Bits hintereinander, da summiert sich der
// Ueberhang auf. Mit einem festen Ziel je Bit bleibt der Fehler bei jedem Bit
// derselbe, statt sich aufzuaddieren.
inline void sleepUntilMs(std::uint64_t deadline) {
    for (;;) {
        const std::uint64_t now = nowMs();
        if (now >= deadline) return;
        const std::uint64_t remaining = deadline - now;
        // Das letzte Stueck aktiv abwarten, sonst verschlaeft man den Zeitpunkt.
        if (remaining > 3) {
            std::this_thread::sleep_for(std::chrono::milliseconds(remaining - 2));
        } else {
            std::this_thread::yield();
        }
    }
}

}  // namespace caruspex
