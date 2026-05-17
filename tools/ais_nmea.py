"""AIS NMEA Type 1 VDM encoder — matches edgesentry-ingest ais_nmea.rs (test helper)."""

from __future__ import annotations


def _push_bits(bits: list[int], value: int, length: int) -> None:
    for i in range(length - 1, -1, -1):
        bits.append((value >> i) & 1)


def _bits_to_payload(bits: list[int]) -> str:
    payload: list[str] = []
    i = 0
    while i + 6 <= len(bits):
        v = 0
        for j in range(6):
            v = (v << 1) | bits[i + j]
        c = v + 48 + (8 if v + 48 >= 88 else 0)
        payload.append(chr(c))
        i += 6
    return "".join(payload)


def nmea_checksum(sentence: str) -> int:
    body = sentence.lstrip("!").split("*", 1)[0]
    cs = 0
    for b in body.encode("ascii"):
        cs ^= b
    return cs


def encode_type1(
    mmsi: int,
    lat: float,
    lon: float,
    sog_knots: float = 0.0,
    cog_deg: float = 0.0,
) -> str:
    """Encode AIS position report type 1 as !AIVDM,...*CS."""
    bits: list[int] = []
    _push_bits(bits, 1, 6)  # message type
    _push_bits(bits, 0, 2)  # repeat indicator
    _push_bits(bits, mmsi, 30)
    _push_bits(bits, 0, 4)  # nav status
    _push_bits(bits, 0, 8)  # rate of turn
    _push_bits(bits, int(round(sog_knots * 10.0)), 10)
    _push_bits(bits, 0, 1)  # position accuracy
    lon_raw = int(round(lon * 600_000.0))
    _push_bits(bits, lon_raw & ((1 << 28) - 1), 28)
    lat_raw = int(round(lat * 600_000.0))
    _push_bits(bits, lat_raw & ((1 << 27) - 1), 27)
    _push_bits(bits, int(round(cog_deg * 10.0)), 12)
    _push_bits(bits, 0, 9)  # true heading
    _push_bits(bits, 0, 6)  # timestamp
    _push_bits(bits, 0, 2)  # special manoeuvre
    _push_bits(bits, 0, 3)  # spare
    _push_bits(bits, 0, 1)  # RAIM
    _push_bits(bits, 0, 19)  # radio status

    payload = _bits_to_payload(bits)
    body = f"AIVDM,1,1,,A,{payload},0"
    cs = nmea_checksum(f"!{body}")
    return f"!{body}*{cs:02X}"


def parse_vdm_checksum_ok(line: str) -> bool:
    line = line.strip()
    star = line.rfind("*")
    if star < 0 or star + 3 != len(line):
        return False
    expected = int(line[star + 1 :], 16)
    return nmea_checksum(line) == expected
