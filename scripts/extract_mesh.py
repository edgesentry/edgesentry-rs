#!/usr/bin/env python3
"""IfcOpenShell mesh extraction sidecar for ``eds inspect extract-mesh``.

Reads an IFC file and writes all product geometry as a triangulated mesh to a
``reference.json`` file consumed by the Inspect App Three.js viewer.

Usage
-----
    python3 extract_mesh.py --ifc <input.ifc> --out <reference.json>

Prerequisites
-------------
    pip install ifcopenshell

Output format
-------------
    {
      "vertices": [[x, y, z], ...],
      "faces":    [[i, j, k], ...]
    }

Coordinates are in metres (world coordinate system).
"""

from __future__ import annotations

import argparse
import json
import sys
from typing import List


def extract_mesh(ifc_path: str, out_path: str) -> None:
    try:
        import ifcopenshell
        import ifcopenshell.geom
    except ImportError:
        print(
            "error: ifcopenshell is not installed\n"
            "  Install it with: pip install ifcopenshell",
            file=sys.stderr,
        )
        sys.exit(1)

    ifc = ifcopenshell.open(ifc_path)

    settings = ifcopenshell.geom.settings()
    settings.set(settings.USE_WORLD_COORDS, True)

    all_vertices: List[List[float]] = []
    all_faces: List[List[int]] = []
    offset = 0

    for product in ifc.by_type("IfcProduct"):
        if not product.Representation:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, product)
        except Exception:
            continue

        verts = shape.geometry.verts   # flat: [x0,y0,z0, x1,y1,z1, ...]
        faces = shape.geometry.faces   # flat: [i0,j0,k0, i1,j1,k1, ...]

        n_verts = len(verts) // 3
        for i in range(n_verts):
            all_vertices.append([verts[i * 3], verts[i * 3 + 1], verts[i * 3 + 2]])

        n_faces = len(faces) // 3
        for i in range(n_faces):
            all_faces.append(
                [
                    faces[i * 3] + offset,
                    faces[i * 3 + 1] + offset,
                    faces[i * 3 + 2] + offset,
                ]
            )

        offset += n_verts

    mesh = {"vertices": all_vertices, "faces": all_faces}

    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(mesh, f)

    print(
        f"Extracted {len(all_vertices):,} vertices and "
        f"{len(all_faces):,} triangles → {out_path}"
    )


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Extract IFC geometry to reference.json (IfcOpenShell sidecar)"
    )
    parser.add_argument("--ifc", required=True, metavar="FILE", help="Input IFC file")
    parser.add_argument(
        "--out", required=True, metavar="FILE", help="Output reference.json path"
    )
    args = parser.parse_args()
    extract_mesh(args.ifc, args.out)


if __name__ == "__main__":
    main()
