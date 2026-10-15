"""Simulated ELM327 with a simulated A2 engine control unit (K-Line/KWP).

Only for testing the software without a car:

    python tools/mock_elm327.py                 # listens on 127.0.0.1:35000
    caruspex_cli read --tcp 127.0.0.1:35000     # console tool
    caruspex --tcp 127.0.0.1:35000              # GUI, connects automatically
"""

from __future__ import annotations

import argparse
import socket
import threading

# Simulated fault memory (throttle body scenario).
STORED = ["0507", "1559"]      # P0507, P1559
PENDING = ["0121"]             # P0121
STATE = {"stored": list(STORED), "pending": list(PENDING)}

# Mode 06 without CAN: test ID -> (component, value, lower limit, upper limit),
# raw 16-bit values. Tests 01-05 are O2 sensor tests at 0.005 V / 0.004 s per
# bit; the evaporative leak check is deliberately out of limits.
MONITOR = {
    0x01: (0x01, 150, 80, 200),     # lean-to-rich threshold 0.750 V
    0x02: (0x01, 120, 50, 180),     # rich-to-lean threshold 0.600 V
    0x05: (0x01, 50, 0, 100),       # switch time 0.200 s
    0x21: (0x21, 194, 0, 200),      # catalyst efficiency
    0x31: (0x31, 320, 0, 256),      # evaporative system leak check: fails
}

# Mode 05, O2 sensor test: (test ID, sensor) -> raw byte.
OXYGEN = {
    (0x01, 1): 140, (0x01, 2): 145, (0x02, 1): 120,
    (0x03, 1): 10, (0x04, 1): 180, (0x05, 1): 25,
}

# Mode 09 identification. The VIN (0902) stays unanswered, like on the real
# 2001 A2; calibration ID and ECU name are there.
IDENTITY = {"04": "036906032G 0001", "0A": "ECM-EngineControl"}

PID_VALUES = {
    "00": "BE1FA813",
    "01": "8107E100",          # MIL on, 7 faults
    "20": "A007B011",
    "03": "0200",
    "04": "45",
    "05": "5A",                # 50 C
    "06": "80",
    "07": "85",
    "0B": "63",
    "0C": "0FA0",              # 1000 /min
    "0D": "00",
    "0F": "37",
    "10": "0113",
    "11": "1F",
    "40": "40000000",          # supports PID 42 (battery voltage)
    "42": "36B0",              # 14.0 V
}


def _dtc_frames(codes: list[str]) -> str:
    if not codes:
        return "00" * 6
    return "".join(codes).ljust(12, "0")[:12]


def handle_command(cmd: str) -> str:
    c = cmd.replace(" ", "").upper()
    if c.startswith("AT"):
        rest = c[2:]
        if rest.startswith("Z"):
            return "ELM327 v1.5"
        if rest.startswith("DP"):
            return "ISO 14230-4 KWP FAST"
        if rest.startswith("I"):
            return "ELM327 v1.5"
        if rest.startswith("SP") or rest in ("E0", "L0", "S0", "H0", "AT1", "PC") \
                or rest.startswith("ST"):
            return "OK"
        return "OK"
    if c.startswith("0100"):
        return "41" + "00" + PID_VALUES["00"]
    if c.startswith("01"):
        pid = c[2:4]
        if pid in PID_VALUES:
            return "41" + pid + PID_VALUES[pid]
        return "NO DATA"
    if c == "03":
        return "43" + _dtc_frames(STATE["stored"])
    if c == "07":
        return "47" + _dtc_frames(STATE["pending"])
    if c == "0A":
        return "NO DATA"
    if c == "04":
        STATE["stored"].clear()
        STATE["pending"].clear()
        PID_VALUES["01"] = "0007E100"
        return "44"
    if c.startswith("02"):
        pid = c[2:4]
        if pid == "02":
            return "4202" + "00" + "0507"
        if pid in PID_VALUES:
            return "42" + pid + "00" + PID_VALUES[pid]
        return "NO DATA"
    if c.startswith("06") and len(c) == 4:
        test = int(c[2:4], 16)
        if test in MONITOR:
            comp, value, low, high = MONITOR[test]
            return f"46{test:02X}{comp:02X}{value:04X}{low:04X}{high:04X}"
        return "NO DATA"
    if c.startswith("05") and len(c) == 6:
        key = (int(c[2:4], 16), int(c[4:6], 16))
        if key in OXYGEN:
            return f"45{key[0]:02X}{key[1]:02X}{OXYGEN[key]:02X}"
        return "NO DATA"
    if c.startswith("09"):
        item = c[2:4]
        if item in IDENTITY:
            return "49" + item + "01" + IDENTITY[item].encode().hex().upper()
        return "NO DATA"
    return "NO DATA"


def serve_client(conn: socket.socket) -> None:
    buf = b""
    with conn:
        conn.sendall(b"\r\r>")
        while True:
            try:
                data = conn.recv(256)
            except OSError:
                return
            if not data:
                return
            buf += data
            while b"\r" in buf:
                line, _, buf = buf.partition(b"\r")
                cmd = line.decode("ascii", "ignore").strip()
                if not cmd:
                    continue
                reply = handle_command(cmd)
                conn.sendall(reply.encode() + b"\r\r>")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=35000)
    a = ap.parse_args()
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind((a.host, a.port))
    srv.listen(2)
    print(f"Mock ELM327 running on {a.host}:{a.port} (Ctrl+C to quit)")
    while True:
        conn, _ = srv.accept()
        threading.Thread(target=serve_client, args=(conn,), daemon=True).start()


if __name__ == "__main__":
    main()
