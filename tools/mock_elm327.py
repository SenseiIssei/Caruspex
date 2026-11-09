"""Simulated ELM327 with simulated control units.

Only for testing the software without a car:

    python tools/mock_elm327.py                 # A2 engine on K-line (KWP), 127.0.0.1:35000
    python tools/mock_elm327.py --can           # CAN 11 bit: engine 7E8 and transmission 7E9
    caruspex_cli read --tcp 127.0.0.1:35000     # console tool
    caruspex --tcp 127.0.0.1:35000              # GUI, connects automatically

The adapter part understands the AT commands Caruspex sends, including ATH1
(headers on). With headers the answers carry the sender: on CAN the 11-bit
identifier plus the ISO-TP frame bytes, on the K-line the KWP header and a
checksum, exactly the way a real ELM327 prints them.
"""

from __future__ import annotations

import argparse
import socket
import threading

# ---------------------------------------------------------------------------
# The simulated control units
# ---------------------------------------------------------------------------

# Engine control unit. Mode 01 values as hex strings without the PID byte.
ENGINE_PIDS = {
    "01": "8107E100",          # MIL on, 1 fault, spark ignition
    "03": "0200",              # closed loop
    "04": "45",
    "05": "5A",                # 50 C
    "06": "80",
    "07": "85",
    "0B": "63",
    "0C": "0FA0",              # 1000 /min
    "0D": "00",
    "0E": "8C",                # 6 deg before TDC
    "0F": "37",
    "10": "0113",
    "11": "1F",
    "13": "33",                # O2 sensors bank 1 and 2, sensors 1 and 2
    "14": "6E80",              # 0.55 V, trim 0 %
    "15": "A0FF",              # 0.80 V, not used for trimming
    "1C": "06",                # EOBD
    "1F": "0258",              # 600 s since start
    "21": "0000",
    "24": "80004000",          # lambda 1.0, 2.0 V
    "2F": "99",                # tank 60 %
    "30": "12",
    "31": "04D2",              # 1234 km since clearing
    "33": "62",                # 98 kPa
    "3C": "1388",              # catalyst 460 C
    "42": "36B0",              # 14.0 V
    "44": "8000",
    "46": "2D",                # 5 C outside
    "49": "26",
    "4D": "0000",
    "4E": "0384",
    "51": "01",                # gasoline
    "5C": "6E",                # oil 70 C
    "5E": "0028",              # 2 l/h
    "67": "035A5B",            # coolant sensors 50 C and 51 C
    "68": "014B",              # intake air sensor 1: 35 C
    "70": "130C800C0000000000" + "02",  # boost 100 kPa wanted, 96 kPa actual, closed loop
    "7F": "01" + "0036EE80" + "00000000" + "00000000",   # 1000 h total
    "A6": "0001E240",          # odometer 12345.6 km
}

# Transmission control unit, only on CAN.
TRANSMISSION_PIDS = {
    "01": "00040000",
    "0D": "00",
    "A4": "03300BB8",          # third gear, ratio 3.000
}

# Simulated fault memory (throttle body scenario on the engine).
STATE = {
    "engine_stored": ["0507", "1559"],     # P0507, P1559
    "engine_pending": ["0121"],            # P0121
    "transmission_stored": ["0715"],       # P0715 input speed sensor
    "headers": False,
    "protocol": "0",
}

# Mode 06 without CAN: test ID -> (component, value, lower limit, upper limit),
# raw 16-bit values. Tests 01-05 are O2 sensor tests at 0.005 V / 0.004 s per
# bit; the evaporative leak check is deliberately out of limits.
MONITOR_KLINE = {
    0x01: (0x01, 150, 80, 200),     # lean-to-rich threshold 0.750 V
    0x02: (0x01, 120, 50, 180),     # rich-to-lean threshold 0.600 V
    0x05: (0x01, 50, 0, 100),       # switch time 0.200 s
    0x21: (0x21, 194, 0, 200),      # catalyst efficiency
    0x31: (0x31, 320, 0, 256),      # evaporative system leak check: fails
}

# Mode 06 on CAN: OBDMID -> list of (TID, unit and scaling ID, value, min, max).
# Cylinder 2 misfires a little.
MONITOR_CAN = {
    0x01: [(0x01, 0x0B, 450, 300, 600), (0x02, 0x0B, 400, 250, 550)],
    0x21: [(0x80, 0x20, 0x40, 0x00, 0xB3)],
    0xA1: [(0x0B, 0x24, 1, 0, 0xFFFF), (0x0C, 0x24, 3, 0, 0xFFFF)],
    0xA2: [(0x0B, 0x24, 0, 0, 0xFFFF), (0x0C, 0x24, 0, 0, 0xFFFF)],
    0xA3: [(0x0B, 0x24, 1, 0, 0xFFFF), (0x0C, 0x24, 3, 0, 0xFFFF)],
    0xA4: [(0x0B, 0x24, 0, 0, 0xFFFF), (0x0C, 0x24, 0, 0, 0xFFFF)],
    0xA5: [(0x0B, 0x24, 0, 0, 0xFFFF), (0x0C, 0x24, 0, 0, 0xFFFF)],
}

# Mode 05, O2 sensor test: (test ID, sensor) -> raw byte. K-line only.
OXYGEN = {
    (0x01, 1): 140, (0x01, 2): 145, (0x02, 1): 120,
    (0x03, 1): 10, (0x04, 1): 180, (0x05, 1): 25,
}


def text_bytes(text: str, length: int) -> list[int]:
    data = list(text.encode("ascii"))
    return (data + [0] * length)[:length]


# Mode 09 per control unit: InfoType -> (number of items, data bytes).
# The VIN (0902) stays unanswered on the K-line, like on the real 2001 A2.
ENGINE_INFO_KLINE = {
    0x04: (1, list(b"036906032G 0001")),
    0x0A: (1, list(b"ECM-EngineControl")),
}
ENGINE_INFO_CAN = {
    0x02: (1, list(b"WAUZZZ8E57A000123")),
    0x04: (2, text_bytes("03L906018DN", 16) + text_bytes("03L906018XX", 16)),
    0x06: (2, [0x1A, 0x2B, 0x3C, 0x4D, 0x00, 0x00, 0xBE, 0xEF]),
    # In-use performance, spark ignition: 40 OBD cycles, 100 ignitions, then
    # completed / conditions for catalyst, O2, EGR, air, EVAP, secondary O2.
    0x08: (16, [0, 40, 0, 100, 0, 12, 0, 40, 0, 0, 0, 0, 0, 30, 0, 40, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 40]),
    0x0A: (1, text_bytes("ECM", 4) + text_bytes("-EngineControl", 16)),
}
TRANSMISSION_INFO_CAN = {
    0x04: (1, text_bytes("09G927750KM", 16)),
    0x06: (1, [0x55, 0x66, 0x77, 0x88]),
    0x0A: (1, text_bytes("TCM", 4) + list(b"-TransmissionCtl")),
}


def can_mode() -> bool:
    return ARGS.can


def support_mask(values: dict, base: int) -> str:
    """Bit mask of the PIDs (or monitor IDs) base+1 .. base+32."""
    keys = [int(k, 16) if isinstance(k, str) else k for k in values]
    mask = 0
    for k in keys:
        if base < k <= base + 32:
            mask |= 1 << (32 - (k - base))
    if any(k > base + 32 for k in keys):
        mask |= 1     # the next range exists
    return f"{mask:08X}"


def dtc_payload(codes: list[str], with_count: bool) -> list[int]:
    data = []
    for code in codes:
        data += [int(code[:2], 16), int(code[2:], 16)]
    if with_count:
        return [len(codes)] + data
    return (data + [0] * 6)[:max(6, len(data))]


# ---------------------------------------------------------------------------
# Answers. Each handler returns a list of (sender, payload bytes), an empty
# list for "nobody answers", or a plain string for adapter replies.
# ---------------------------------------------------------------------------

def mode01(pid: int, units) -> list:
    out = []
    for name, pids in units:
        if pid % 0x20 == 0:
            if pid == 0 or any(int(k, 16) > pid for k in pids):
                out.append((name, [0x41, pid] + list(bytes.fromhex(support_mask(pids, pid)))))
            continue
        key = f"{pid:02X}"
        if key in pids:
            out.append((name, [0x41, pid] + list(bytes.fromhex(pids[key]))))
    return out


def handle_obd(c: str) -> list:
    can = can_mode()
    units = [("engine", ENGINE_PIDS)]
    if can:
        units.append(("transmission", TRANSMISSION_PIDS))

    if c.startswith("01") and len(c) >= 4:
        return mode01(int(c[2:4], 16), units)
    if c == "03":
        out = [("engine", [0x43] + dtc_payload(STATE["engine_stored"], can))]
        if can:
            out.append(("transmission", [0x43] + dtc_payload(STATE["transmission_stored"], True)))
        return out
    if c == "07":
        return [("engine", [0x47] + dtc_payload(STATE["engine_pending"], can))]
    if c == "0A":
        return []
    if c == "04":
        STATE["engine_stored"].clear()
        STATE["engine_pending"].clear()
        STATE["transmission_stored"].clear()
        ENGINE_PIDS["01"] = "0007E100"
        return [(name, [0x44]) for name, _ in units]
    if c.startswith("02") and len(c) >= 4:
        pid = c[2:4]
        if pid == "02":
            return [("engine", [0x42, 0x02, 0x00, 0x05, 0x07])]
        if pid in ENGINE_PIDS:
            return [("engine", [0x42, int(pid, 16), 0x00] + list(bytes.fromhex(ENGINE_PIDS[pid])))]
        return []
    if c.startswith("06") and len(c) == 4:
        tid = int(c[2:4], 16)
        if can:
            if tid % 0x20 == 0:
                return [("engine", [0x46, tid] + list(bytes.fromhex(support_mask(MONITOR_CAN, tid))))]
            if tid not in MONITOR_CAN:
                return []
            data = [0x46]
            for test, uas, value, low, high in MONITOR_CAN[tid]:
                data += [tid, test, uas, value >> 8, value & 0xFF, low >> 8, low & 0xFF,
                         high >> 8, high & 0xFF]
            return [("engine", data)]
        if tid in MONITOR_KLINE:
            comp, value, low, high = MONITOR_KLINE[tid]
            return [("engine", [0x46, tid, comp, value >> 8, value & 0xFF, low >> 8, low & 0xFF,
                                high >> 8, high & 0xFF])]
        return []
    if c.startswith("05") and len(c) == 6 and not can:
        key = (int(c[2:4], 16), int(c[4:6], 16))
        if key in OXYGEN:
            return [("engine", [0x45, key[0], key[1], OXYGEN[key]])]
        return []
    if c.startswith("09") and len(c) >= 4:
        item = int(c[2:4], 16)
        out = []
        tables = [("engine", ENGINE_INFO_CAN if can else ENGINE_INFO_KLINE)]
        if can:
            tables.append(("transmission", TRANSMISSION_INFO_CAN))
        for name, table in tables:
            if item in table:
                count, data = table[item]
                out.append((name, [0x49, item, count] + data))
        return out
    return []


def handle_at(rest: str) -> str:
    if rest.startswith("Z") or rest.startswith("WS"):
        STATE["headers"] = False
        return "ELM327 v1.5"
    if rest.startswith("I"):
        return "ELM327 v1.5"
    if rest == "H1":
        STATE["headers"] = True
    elif rest == "H0":
        STATE["headers"] = False
    elif rest.startswith("SP"):
        STATE["protocol"] = rest[2:3]
    elif rest.startswith("DPN"):
        return "6" if ARGS.can else "5"
    elif rest.startswith("DP"):
        return "ISO 15765-4 (CAN 11/500)" if ARGS.can else "ISO 14230-4 KWP FAST"
    return "OK"


# ---------------------------------------------------------------------------
# Printing the way an ELM327 does (ATS0: no spaces)
# ---------------------------------------------------------------------------

CAN_IDS = {"engine": "7E8", "transmission": "7E9"}
KLINE_SOURCES = {"engine": 0x10, "transmission": 0x18}


def hexs(data) -> str:
    return "".join(f"{b:02X}" for b in data)


def format_can(sender: str, payload: list[int]) -> list[str]:
    if not STATE["headers"]:
        if len(payload) <= 7:
            return [hexs(payload)]
        lines = [f"{len(payload):03X}", "0:" + hexs(payload[:6])]
        rest, index = payload[6:], 1
        while rest:
            lines.append(f"{index & 0xF:X}:" + hexs(rest[:7]))
            rest, index = rest[7:], index + 1
        return lines
    can_id = CAN_IDS[sender]
    if len(payload) <= 7:
        frames = [[len(payload)] + payload]
    else:
        frames = [[0x10 | (len(payload) >> 8), len(payload) & 0xFF] + payload[:6]]
        rest, seq = payload[6:], 1
        while rest:
            frames.append([0x20 | (seq & 0x0F)] + rest[:7])
            rest, seq = rest[7:], seq + 1
    return [can_id + hexs(f + [0xCC] * (8 - len(f))) for f in frames]


def format_kline(sender: str, payload: list[int]) -> list[str]:
    if not STATE["headers"]:
        return [hexs(payload)]
    frame = [0x80 | len(payload), 0xF1, KLINE_SOURCES[sender]] + payload
    return [hexs(frame + [sum(frame) & 0xFF])]


def handle_command(cmd: str) -> str:
    c = cmd.replace(" ", "").upper()
    if c.startswith("AT"):
        return handle_at(c[2:])
    # Without the right protocol selected nobody answers.
    if STATE["protocol"] not in ("0", ("6" if ARGS.can else "5")):
        return "NO DATA" if ARGS.can else "UNABLE TO CONNECT"
    answers = handle_obd(c)
    if not answers:
        return "NO DATA"
    lines = []
    for sender, payload in answers:
        lines += format_can(sender, payload) if can_mode() else format_kline(sender, payload)
    return "\r".join(lines)


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


ARGS = argparse.Namespace(can=False)


def main() -> None:
    global ARGS
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=35000)
    ap.add_argument("--can", action="store_true",
                    help="CAN 11 bit with engine (7E8) and transmission (7E9) instead of K-line")
    ARGS = ap.parse_args()
    srv = socket.socket()
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind((ARGS.host, ARGS.port))
    srv.listen(2)
    kind = "CAN 11 bit, engine 7E8 + transmission 7E9" if ARGS.can else "K-line KWP, engine only"
    print(f"Mock ELM327 running on {ARGS.host}:{ARGS.port} ({kind}, Ctrl+C to quit)")
    while True:
        conn, _ = srv.accept()
        threading.Thread(target=serve_client, args=(conn,), daemon=True).start()


if __name__ == "__main__":
    main()
