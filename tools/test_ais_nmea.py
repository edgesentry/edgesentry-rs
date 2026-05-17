"""Unit tests for tools/ais_nmea.py (issue #404)."""

from __future__ import annotations

import re
import sys
import unittest
from pathlib import Path

# Allow `from ais_nmea import ...` when run from repo root or tools/
sys.path.insert(0, str(Path(__file__).resolve().parent))

from ais_nmea import encode_type1, nmea_checksum, parse_vdm_checksum_ok  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[1]
SG_MARITIME = (
    REPO_ROOT / "crates/edgesentry-ingest/fixtures/sg_maritime_ais.nmea"
)
SG_STRAIT = REPO_ROOT / "demo/sg-strait-15min.nmea"


def _nmea_lines(path: Path) -> list[str]:
    return [
        line.strip()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip().startswith("!AIVDM")
    ]


class TestAisNmeaEncode(unittest.TestCase):
    def test_encode_produces_valid_checksum(self) -> None:
        line = encode_type1(563_012_345, 1.2658, 103.8200, 5.0, 0.0)
        self.assertTrue(line.startswith("!AIVDM,1,1,,A,"))
        self.assertRegex(line, r"\*[0-9A-Fa-f]{2}$")
        self.assertTrue(parse_vdm_checksum_ok(line))

    def test_checksum_helper_matches_nmea_spec(self) -> None:
        body = "AIVDM,1,1,,A,18HsRv@00j7K@100f<a000000000,0"
        line = f"!{body}*1C"
        self.assertEqual(nmea_checksum(line), 0x1C)

    def test_sg_maritime_fixture_lines_have_valid_checksum(self) -> None:
        lines = _nmea_lines(SG_MARITIME)
        self.assertEqual(len(lines), 5)
        for line in lines:
            self.assertTrue(
                parse_vdm_checksum_ok(line),
                f"bad checksum: {line}",
            )

    def test_sg_strait_fixture_lines_have_valid_checksum(self) -> None:
        lines = _nmea_lines(SG_STRAIT)
        self.assertEqual(len(lines), 30)
        for line in lines:
            self.assertTrue(parse_vdm_checksum_ok(line))


if __name__ == "__main__":
    unittest.main()
