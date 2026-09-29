"""Reads each design document named on the command line with the generated types, and prints a
line for each: what it holds, or why the reader refused it. Exits 1 if it refused any.

    python3 schema/format/python/read_design.py design.hpr [more.hpr ...]

Needs Python 3.11 or later.
"""

import sys
from pathlib import Path

from hpr_design import Component, DesignFormatError, read_design


def count(components: list[Component]) -> int:
    """How many parts `components` hold, counting the parts inside parts."""
    return sum(1 + count(c.get("children", [])) for c in components)


def main(paths: list[str]) -> int:
    refused = 0
    for path in paths:
        try:
            design = read_design(Path(path).read_text(encoding="utf-8"))
        except DesignFormatError as error:
            refused += 1
            print(f"refused {path}: {error}")
            continue
        stages = design["rocket"]["stages"]
        parts = sum(count(stage["components"]) for stage in stages)
        configurations = len(design["motors"]["configurations"])
        print(
            f"read {path}: stages {len(stages)}, parts {parts},"
            f" motor configurations {configurations}"
        )
    return 0 if refused == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
