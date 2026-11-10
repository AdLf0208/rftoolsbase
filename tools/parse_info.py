#!/usr/bin/env python3
"""Parse `/rftoolsbase info`'s line into shell assignments.

`Machine Infuser at [200, 101, 0]: 600/60000 FE, infusing 0/5, block infused 0/256,
shard rftoolsbase:dimensionalshard x1, target rftoolsbase:machine_infuser x1 infused 1, t=264`

Prints `ENERGY=… CAPACITY=… INFUSING=… BLOCK_INFUSED=… SHARD_COUNT=…
TARGET_LEVEL=… T=…` (or `ok=0`), for the e2e to `eval`.
"""

from __future__ import annotations

import re
import sys

LINE = re.compile(
    r"^Machine Infuser at \[.*?\]: (?P<energy>-?\d+)/(?P<capacity>\d+) FE, "
    r"infusing (?P<infusing>-?\d+)/(?P<maxinfusing>-?\d+), "
    r"block infused (?P<block_infused>-?\d+)/(?P<maxinfuse>-?\d+), "
    r"shard (?P<shard>.*?), target (?P<target>.*?), t=(?P<t>\d+)(?: nonce=\d+)?$"
)
STACK = re.compile(r"(?P<item>\S+) x(?P<count>\d+)")
TARGET = re.compile(r"(?P<item>\S+) x(?P<count>\d+) infused (?P<level>-?\d+)")


def main() -> None:
    line = sys.argv[1] if len(sys.argv) > 1 else ""
    match = LINE.match(line)
    if not match:
        print("ok=0")
        return
    print("ok=1")
    print(f"ENERGY={match['energy']}")
    print(f"CAPACITY={match['capacity']}")
    print(f"INFUSING={match['infusing']}")
    print(f"MAX_INFUSING={match['maxinfusing']}")
    print(f"BLOCK_INFUSED={match['block_infused']}")
    print(f"MAX_INFUSE={match['maxinfuse']}")
    print(f"T={match['t']}")
    print(f"SHARD={match['shard']!r}")
    shard = STACK.match(match["shard"])
    print(f"SHARD_COUNT={shard['count'] if shard else 0}")
    target = TARGET.match(match["target"])
    print(f"TARGET={match['target']!r}")
    print(f"TARGET_COUNT={target['count'] if target else 0}")
    print(f"TARGET_LEVEL={target['level'] if target else -1}")


if __name__ == "__main__":
    main()
