#!/usr/bin/env python3
"""Generate demo/sg-strait-15min.nmea — Singapore Strait approach track (issue #404)."""

from __future__ import annotations

import argparse
from pathlib import Path

from ais_nmea import encode_type1

MMSI = 563_012_345
LON = 103.8200
SOG = 5.0
COG = 0.0
STEP_LAT = 0.001697  # matches y=200→389 m step in sg_maritime_ais.nmea
INTERVAL_S = 30
DURATION_MIN = 15

REPO_ROOT = Path(__file__).resolve().parents[1]
CANONICAL = REPO_ROOT / "crates/edgesentry-ingest/fixtures/sg_maritime_ais.nmea"


def _canonical_sentences() -> list[str]:
    lines: list[str] = []
    with CANONICAL.open(encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if line.startswith("!AIVDM"):
                lines.append(line)
    if len(lines) != 5:
        raise SystemExit(f"expected 5 sentences in {CANONICAL}, got {len(lines)}")
    return lines


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "-o",
        "--out",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "demo" / "sg-strait-15min.nmea",
    )
    args = parser.parse_args()

    lines: list[str] = [
        "# Singapore Strait AIS fixture — MMSI=563012345 approaching restricted zone",
        "# Port reference: lat=1.2640, lon=103.8200 (sg-maritime-security params.toml)",
        "# 30 position reports, 30 s apart (~15 min wall time at 1× replay)",
        "# Sentence 5+ enters RESTRICTED_ZONE_APPROACH (y > 200 m port-local)",
        "",
    ]

    count = (DURATION_MIN * 60) // INTERVAL_S
    canonical = _canonical_sentences()
    lines.extend(canonical)
    # Continue north from inside-zone position (last canonical sentence)
    tail_lat = 1.2640 + 389 / 111_320.0
    for i in range(len(canonical), count):
        lat = tail_lat + (i - len(canonical) + 1) * STEP_LAT
        lines.append(encode_type1(MMSI, lat, LON, SOG, COG))

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Wrote {count} sentences to {args.out}")


if __name__ == "__main__":
    main()
