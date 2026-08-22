"""Simulierter ELM327 mit einem simulierten A2-Motorsteuergeraet (K-Line/KWP).

Nur zum Testen der Software ohne Auto:

    python tools/mock_elm327.py            # lauscht auf 127.0.0.1:35000
    python -m a2obd --tcp 127.0.0.1:35000 read
"""

from __future__ import annotations

import argparse
import socket
import threading

# Simulierter Fehlerspeicher (Drosselklappen-Szenario).
STORED = ["0507", "1559"]      # P0507, P1559
PENDING = ["0121"]             # P0121
STATE = {"stored": list(STORED), "pending": list(PENDING)}

PID_VALUES = {
    "00": "BE1FA813",
    "01": "8107E100",          # MIL an, 7 Fehler
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
    "42": "36B0",              # 14,0 V
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
    if c.startswith("09"):
        return "NO DATA"          # wie beim echten A2 von 2001
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
    print(f"Mock-ELM327 laeuft auf {a.host}:{a.port} (Strg+C beendet)")
    while True:
        conn, _ = srv.accept()
        threading.Thread(target=serve_client, args=(conn,), daemon=True).start()


if __name__ == "__main__":
    main()
