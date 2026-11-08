"""Make a clip of the dock being used.

A browser with no window will not hand back the frames of an animation, so
the clip is made the other way round: the page is asked to play a gesture up
to a point and hold there, a still is taken, and the next run goes one step
further. Every run starts from the same layout and plays the same steps, so
the stills line up into a film of one continuous gesture.

    tools/clip.py <out.gif>

Needs `trunk serve` already running.
"""

import subprocess
import sys
import urllib.parse
from pathlib import Path

WIDTH, HEIGHT = 1900, 1200
PORT = 7150
EVERY = 90  # Milliseconds a frame is held for, in the finished clip.

out = Path(sys.argv[1] if len(sys.argv) > 1 else "clip.gif")
shots = out.parent / "shots"
shots.mkdir(parents=True, exist_ok=True)


def path(start, end, steps):
    """Points along a line, easing out, so a pointer does not jerk off."""
    for i in range(1, steps + 1):
        at = i / steps
        at = 1 - (1 - at) ** 2

        yield (
            round(start[0] + (end[0] - start[0]) * at),
            round(start[1] + (end[1] - start[1]) * at),
        )


# One gesture, written as the steps it is made of. A frame is the gesture up
# to and including that step, so the list below is also the storyboard.
steps: list[tuple[str, int, int]] = []
frames: list[list[tuple[str, int, int]]] = []


def hold(times=1):
    for _ in range(times):
        frames.append(list(steps))


def press(x, y):
    steps.append(("down", x, y))
    hold()


def drag(to, steps_in_between=7, at=None):
    start = (steps[-1][1], steps[-1][2]) if at is None else at

    for point in path(start, to, steps_in_between):
        steps.append(("move", *point))
        hold()


def release():
    steps.append(("up", steps[-1][1], steps[-1][2]))
    hold(3)


def click(x, y):
    press(x, y)
    steps.append(("up", x, y))
    hold()


# The layout to start from, so a reader sees it before anything moves.
#
# Every point below is in the dock's own coordinates, which start under the
# page's own bar — not the page's. Read them off a still rather than guessing:
# a tab is 27px tall, and a press four pixels low lands in the window instead.
hold(4)

# Leaf through the slot the editor shares with the test run.
click(465, 13)
hold(4)
click(300, 13)
hold(4)

# Take the test run out of that slot and give it a side of its own.
press(465, 13)
drag((900, 320), 6)
drag((1500, 560), 7)
hold(3)
release()
hold(2)

# Make room for it.
press(928, 700)
drag((1180, 700), 6)
hold(2)
release()
hold(3)

# And put it back where it came from, after the editor rather than before it.
press(1250, 13)
drag((900, 320), 6)
drag((420, 13), 7)
hold(3)
release()
hold(7)

print(f"{len(frames)} frames", flush=True)

for i, gesture in enumerate(frames):
    name = shots / f"frame-{i:03d}.png"
    script = "|".join(f"{what}:{x},{y}" for what, x, y in gesture)
    query = urllib.parse.urlencode({"still": "1", "gesture": script}, safe=":,|")

    subprocess.run(
        [
            "google-chrome-stable",
            "--headless",
            "--disable-gpu",
            "--hide-scrollbars",
            f"--window-size={WIDTH},{HEIGHT}",
            f"--screenshot={name}",
            f"--virtual-time-budget={1400 + 100 * len(gesture)}",
            f"http://127.0.0.1:{PORT}/?{query}",
        ],
        check=True,
        capture_output=True,
    )

    print(f"\r{i + 1}/{len(frames)}", end="", flush=True)

print("\nassembling")

subprocess.run(
    ["magick", "-delay", str(EVERY // 10), "-loop", "0",
     *sorted(str(p) for p in shots.glob("frame-*.png")),
     "-resize", "1100x", "-colors", "64", "-layers", "Optimize", str(out)],
    check=True,
)

print(out)
