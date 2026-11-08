"""Read a trace from tools/trace.sh and say what moved, when, and how.

A settle is hard to judge by eye a frame at a time, and the question is
usually about timing: what started when, what finished when, and whether the
way it got there was the same for everything that moved.
"""

import json
import sys

rows = json.load(open(sys.argv[1]) if len(sys.argv) > 1 else sys.stdin)
what = sys.argv[2] if len(sys.argv) > 2 else ""

times = [row[0] for row in rows]
gaps = [b - a for a, b in zip(times, times[1:])]

print(f"{len(rows)} frames over {times[-1]}ms", end="")
if gaps:
    print(f", {min(gaps)}-{max(gaps)}ms apart (median {sorted(gaps)[len(gaps) // 2]})")

# Every name that was ever on screen, and the rectangle it had each frame.
seen: dict[str, dict[int, tuple]] = {}
for at, items in rows:
    for name, *rect in items:
        seen.setdefault(name, {})[at] = tuple(rect)

LABEL = {0: "x", 1: "y", 2: "w", 3: "h"}

for name, frames in seen.items():
    if what and what not in name:
        continue

    first, last = min(frames), max(frames)
    moved = []

    for i in range(4):
        values = [v[i] for v in frames.values()]
        if max(values) - min(values) < 2:
            continue

        start, end = frames[first][i], frames[last][i]
        # When it left the first value, and when it reached the last one.
        off = next((at for at in sorted(frames) if frames[at][i] != start), first)
        on = next((at for at in sorted(frames) if frames[at][i] == end), last)

        moved.append((i, start, end, off, on))

    if not moved:
        continue

    print(f"\n{name}")
    print(f"  on screen {first}-{last}ms")

    for i, start, end, off, on in moved:
        print(f"  {LABEL[i]}: {start} -> {end}, moving {off}-{on}ms ({on - off}ms)")

        # How far along it was, in tenths of the way through the move.
        marks = []
        for step in range(11):
            at = off + (on - off) * step / 10
            near = min(frames, key=lambda f: abs(f - at))
            done = (frames[near][i] - start) / (end - start) if end != start else 1
            marks.append(f"{done:.2f}")

        print("     " + " ".join(marks))
