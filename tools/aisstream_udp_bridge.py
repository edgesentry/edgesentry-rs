#!/usr/bin/env python3
"""
aisstream.io WebSocket → NMEA Type 1 UDP bridge for eds ingest stream (issue #404).

Requires AISSTREAM_API_KEY in the environment (free key at https://aisstream.io).

Example:
  export AISSTREAM_API_KEY=...
  python3 tools/aisstream_udp_bridge.py --host 127.0.0.1 --port 9100
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import socket
import sys

from ais_nmea import encode_type1

WEBSOCKET_URL = "wss://stream.aisstream.io/v0/stream"
# Singapore Strait + Malacca (same as indago pipelines/ingest/ais_stream.py)
DEFAULT_BBOX = [[-5.0, 92.0], [22.0, 122.0]]


def _parse_position_report(msg: dict) -> tuple[int, float, float, float, float] | None:
    if msg.get("MessageType") != "PositionReport":
        return None
    meta = msg.get("MetaData") or {}
    report = (msg.get("Message") or {}).get("PositionReport") or {}
    mmsi_raw = meta.get("MMSI") or report.get("UserID")
    lat = report.get("Latitude") if report.get("Latitude") is not None else meta.get("latitude")
    lon = report.get("Longitude") if report.get("Longitude") is not None else meta.get("longitude")
    if mmsi_raw is None or lat is None or lon is None:
        return None
    try:
        mmsi = int(str(mmsi_raw))
        sog = float(report.get("Sog") or 0.0)
        cog = float(report.get("Cog") or 0.0)
        return mmsi, float(lat), float(lon), sog, cog
    except (TypeError, ValueError):
        return None


async def run_bridge(
    api_key: str,
    host: str,
    port: int,
    bbox: list,
    duration: float,
) -> int:
    try:
        import websockets
    except ImportError:
        print(
            "error: install websockets — pip install -r tools/requirements.txt",
            file=sys.stderr,
        )
        return 1

    subscription = {
        "APIKey": api_key,
        "BoundingBoxes": [bbox],
        "FilterMessageTypes": ["PositionReport"],
    }

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    dest = (host, port)
    sent = 0
    loop = asyncio.get_running_loop()
    deadline = loop.time() + duration if duration > 0 else None

    print(f"Connecting to {WEBSOCKET_URL} → UDP {host}:{port}", flush=True)

    async with websockets.connect(WEBSOCKET_URL) as ws:
        await ws.send(json.dumps(subscription))
        print(f"Subscribed bbox={bbox}", flush=True)
        while True:
            if deadline is not None and loop.time() >= deadline:
                break
            try:
                raw = await asyncio.wait_for(ws.recv(), timeout=30.0)
            except asyncio.TimeoutError:
                continue
            try:
                msg = json.loads(raw)
            except json.JSONDecodeError:
                continue
            parsed = _parse_position_report(msg)
            if parsed is None:
                continue
            mmsi, lat, lon, sog, cog = parsed
            line = encode_type1(mmsi, lat, lon, sog, cog)
            sock.sendto(line.encode("ascii"), dest)
            sent += 1
            if sent % 50 == 0:
                print(f"  forwarded {sent} position reports …", flush=True)

    sock.close()
    print(f"Done — forwarded {sent} NMEA sentences", flush=True)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=9100)
    parser.add_argument(
        "--duration",
        type=float,
        default=0.0,
        help="Stop after N seconds (0 = until Ctrl-C)",
    )
    parser.add_argument(
        "--bbox",
        type=str,
        default="",
        help='JSON bbox e.g. "[[-5,92],[22,122]]" (default: Singapore Strait)',
    )
    args = parser.parse_args()

    api_key = os.environ.get("AISSTREAM_API_KEY", "").strip()
    if not api_key:
        print("error: set AISSTREAM_API_KEY", file=sys.stderr)
        return 1

    bbox = DEFAULT_BBOX
    if args.bbox:
        bbox = json.loads(args.bbox)

    return asyncio.run(run_bridge(api_key, args.host, args.port, bbox, args.duration))


if __name__ == "__main__":
    raise SystemExit(main())
