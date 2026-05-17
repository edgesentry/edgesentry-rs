#!/usr/bin/env python3
"""Replay a .nmea log file over UDP for eds ingest stream (ais://) demos — issue #404."""

from __future__ import annotations

import argparse
import socket
import sys
import time
from pathlib import Path


def iter_nmea_lines(path: Path):
    with path.open(encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            yield line


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "nmea_file",
        type=Path,
        help="NMEA log (.nmea) with !AIVDM sentences",
    )
    parser.add_argument(
        "--host",
        default="127.0.0.1",
        help="UDP destination host (default: 127.0.0.1)",
    )
    parser.add_argument(
        "--port",
        type=int,
        default=9100,
        help="UDP destination port (default: 9100)",
    )
    parser.add_argument(
        "--speed",
        type=float,
        default=1.0,
        help="Replay speed multiplier (2.0 = twice as fast, default: 1.0)",
    )
    parser.add_argument(
        "--interval",
        type=float,
        default=30.0,
        help="Seconds between sentences at 1× speed (default: 30)",
    )
    parser.add_argument(
        "--loop",
        action="store_true",
        help="Loop the file until --duration elapses",
    )
    parser.add_argument(
        "--duration",
        type=float,
        default=0.0,
        help="Stop after N seconds (0 = play file once)",
    )
    args = parser.parse_args()

    if not args.nmea_file.is_file():
        print(f"error: file not found: {args.nmea_file}", file=sys.stderr)
        return 1

    lines = list(iter_nmea_lines(args.nmea_file))
    if not lines:
        print(f"error: no NMEA sentences in {args.nmea_file}", file=sys.stderr)
        return 1

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    dest = (args.host, args.port)
    delay = args.interval / max(args.speed, 0.001)
    deadline = time.monotonic() + args.duration if args.duration > 0 else None
    sent = 0

    print(
        f"Replaying {len(lines)} sentences to {args.host}:{args.port} "
        f"(interval={delay:.2f}s, speed={args.speed}×)",
        flush=True,
    )

    try:
        while True:
            for line in lines:
                if deadline is not None and time.monotonic() >= deadline:
                    print(f"Done — sent {sent} sentences", flush=True)
                    return 0
                sock.sendto(line.encode("ascii"), dest)
                sent += 1
                time.sleep(delay)
            if not args.loop and args.duration <= 0:
                break
            if not args.loop and deadline is None:
                break
    except KeyboardInterrupt:
        print("\nInterrupted", flush=True)
    finally:
        sock.close()

    print(f"Done — sent {sent} sentences", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
