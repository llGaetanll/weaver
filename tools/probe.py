"""Check what the dock looks like against the claims the board makes.

A probe plays a gesture, holds there, and asks two questions of the same
moment: where the things the dock draws are, which the page answers, and what
colour each pixel is, which the picture answers. A claim is written against
the first and proved against the second, so nothing here reads the stylesheet
— a backend that is not a browser answers the same two questions its own way
and the claims come with it.

    tools/probe.py                 check every claim
    tools/probe.py --only shadow   check the claims whose name matches
    tools/probe.py --self-check    break each claim on purpose and watch it fail

Needs `trunk serve` already running. Nothing is left behind.
"""

import json
import re
import subprocess
import tempfile
import sys
import threading
import tomllib
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler
from http.server import HTTPServer
from pathlib import Path

WIDTH, HEIGHT = 1900, 1200
PORT = 7150
BACK = 7152

HERE = Path(__file__).parent
CLAIMS = HERE / "claims.toml"
SHOT = Path(tempfile.gettempdir()) / "weaver-probe.png"


# --- The picture --------------------------------------------------------


class Picture:
    """The still, as pixels that can be asked about by name."""

    def __init__(self, path):
        raw = subprocess.run(
            ["magick", str(path), "-depth", "8", "ppm:-"],
            check=True, capture_output=True,
        ).stdout

        fields = []
        at = 0
        while len(fields) < 4:
            end = raw.index(b"\n", at)
            line = raw[at:end]
            at = end + 1
            if not line.startswith(b"#"):
                fields.extend(line.split())

        self.w, self.h = int(fields[1]), int(fields[2])
        self.body = raw[at:]

    def at(self, x, y):
        if not (0 <= x < self.w and 0 <= y < self.h):
            return None

        i = (y * self.w + x) * 3
        return tuple(self.body[i:i + 3])

    def colours(self, x0, y0, x1, y1):
        """Every distinct colour in a patch, as a set."""
        seen = set()
        for y in range(max(0, y0), min(self.h, y1)):
            row = (y * self.w) * 3
            for x in range(max(0, x0), min(self.w, x1)):
                i = row + x * 3
                seen.add(tuple(self.body[i:i + 3]))

        return seen

    def darkest(self, x0, y0, x1, y1):
        dark = None
        for colour in self.colours(x0, y0, x1, y1):
            if dark is None or sum(colour) < sum(dark):
                dark = colour

        return dark


# --- The moment ---------------------------------------------------------


def catch(port, into):
    """One POST from the page, parsed."""

    class Catch(BaseHTTPRequestHandler):
        def do_POST(self):
            length = int(self.headers.get("content-length", 0))
            into.append(json.loads(self.rfile.read(length).decode()))
            self.send_response(204)
            self.send_header("access-control-allow-origin", "*")
            self.end_headers()

        def log_message(self, *args):
            pass

    server = HTTPServer(("127.0.0.1", port), Catch)
    server.timeout = 30
    server.handle_request()
    server.server_close()


# What a watching browser loses around its page. One taking a picture is
# sized by its page, so the two only agree once this is added back.
FRAME = [0, 0]


def chrome(query, shot=None):
    """A browser on the page, which quits on its own only if it is taking a
    picture — one that is only watching has to be stopped."""
    w, h = (WIDTH, HEIGHT) if shot else (WIDTH + FRAME[0], HEIGHT + FRAME[1])
    args = [
        "google-chrome-stable", "--headless", "--disable-gpu", "--hide-scrollbars",
        "--force-device-scale-factor=1", f"--window-size={w},{h}",
    ]
    if shot:
        args.append(f"--screenshot={shot}")
        args.append(f"--virtual-time-budget={2000 + 100 * query['gesture'].count('|')}")

    args.append(f"http://127.0.0.1:{PORT}/?" + urllib.parse.urlencode(query, safe=":,|"))

    return subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def visit(gesture, break_with=None, hide=None):
    """The gesture, held: where everything is, and a picture of it there.

    Two runs rather than one. A browser taking a screenshot is on a virtual
    clock and quits the moment it has one, which is too early for the page to
    post anything; a browser that posts is on the real one and hands back no
    picture. Both runs are the same still of the same gesture, so they agree.
    """
    base = {"still": "1", "gesture": gesture, "tokens": ",".join(TOKENS)}
    if break_with:
        base["break"] = break_with
    if hide:
        base["hide"] = hide

    seen = watch(base)
    view = seen[0].get("viewport", {"w": WIDTH, "h": HEIGHT})

    if (view["w"], view["h"]) != (WIDTH, HEIGHT):
        FRAME[0] += WIDTH - view["w"]
        FRAME[1] += HEIGHT - view["h"]
        seen = watch(base)

    chrome(base, shot=SHOT).wait(timeout=60)

    return seen[0], Picture(SHOT)


def watch(base):
    """What the page says is where, from a browser that is only watching."""
    seen = []
    listen = threading.Thread(target=catch, args=(BACK, seen))
    listen.start()

    watching = chrome({**base, "probe": "1", "to": str(BACK)})
    listen.join()
    watching.terminate()
    watching.wait()

    if not seen:
        raise SystemExit("the page said nothing — is `trunk serve` running?")

    return seen


TOKENS = ["panel", "sunken", "border", "border-firm", "bg", "drop", "drop-chosen", "text"]


def rgb(value):
    """A colour the host named, as the three numbers it ends up being."""
    value = value.strip()
    if value.startswith("#"):
        value = value[1:]
        if len(value) == 3:
            value = "".join(c * 2 for c in value)
        return tuple(int(value[i:i + 2], 16) for i in (0, 2, 4))

    found = re.findall(r"[\d.]+", value)
    if len(found) >= 3:
        return tuple(int(float(n)) for n in found[:3])

    raise ValueError(f"cannot read the colour {value!r}")


class Moment:
    """What one gesture, held, lets a claim ask about."""

    def __init__(self, said, picture, gesture=None, break_with=None):
        self.picture = picture
        self.gesture = gesture
        self.break_with = break_with
        self.hidden = {}
        self.palette = {name: rgb(value) for name, value in said["tokens"].items() if value}
        self.boxes = {}
        for box in said["boxes"]:
            self.boxes[box["name"]] = box

    def box(self, role):
        """A role's rectangle, in whole pixels, by name or by prefix."""
        if role in self.boxes:
            found = self.boxes[role]
        else:
            like = [name for name in self.boxes if name.split(":")[:2] == role.split(":")]
            if len(like) != 1:
                raise Claimless(f"{role!r} names {len(like)} of the things drawn")
            found = self.boxes[like[0]]

        x, y = round(found["x"]), round(found["y"])
        return x, y, x + round(found["w"]), y + round(found["h"])

    def token(self, name):
        if name not in self.palette:
            raise Claimless(f"the theme has no --{name}")

        return self.palette[name]

    def without(self, role):
        """The same moment, taken again with one of the things left out."""
        if role not in self.hidden:
            _, picture = visit(self.gesture, self.break_with, hide=role)
            self.hidden[role] = picture

        return self.hidden[role]

    def known(self, colour):
        """Whether a pixel is a colour the theme actually names."""
        return colour in set(self.palette.values())


class Claimless(Exception):
    """A claim that could not be asked, which is a failure and not a pass."""


# --- The checks ---------------------------------------------------------
#
# Five, and each is about a thing a person can see. They never read a
# stylesheet: every one of them is a question about the picture, asked at a
# rectangle the backend named.


def check_edge(moment, claim):
    """A one pixel line down a side of a role — or the want of one."""
    x0, y0, x1, y1 = moment.box(claim["role"])
    side = claim["side"]
    want = claim["is"]
    inset = claim.get("inset", 2)

    if side in ("left", "right"):
        x = x0 if side == "left" else x1 - 1
        line = [(x, y) for y in range(y0 + inset, y1 - inset)]
        inward = (1 if side == "left" else -1, 0)
    else:
        y = y0 if side == "top" else y1 - 1
        line = [(x, y) for x in range(x0 + inset, x1 - inset)]
        inward = (0, 1 if side == "top" else -1)

    if len(line) < 3:
        raise Claimless(f"the {side} of {claim['role']} is {len(line)}px long")

    seen = {moment.picture.at(x, y) for x, y in line}
    beside = {moment.picture.at(x + inward[0], y + inward[1]) for x, y in line}

    if want == "none":
        if seen != beside:
            return f"the {side} of {claim['role']} is not the same as what is beside it"
        return None

    wanted = moment.token(want)
    if seen != {wanted}:
        return f"the {side} of {claim['role']} is {sorted(seen)}, not --{want} {wanted}"
    if beside == {wanted}:
        return f"the {side} of {claim['role']} is {len(beside)}px thick, not one"

    return None


def check_outside(moment, claim):
    """Nothing is painted outside a role: no shadow, no glow, no halo.

    Asked by taking the same moment again without the thing in it. Anything
    that differs outside its own rectangle is paint it put there, which is
    what a shadow is. Reading the colours around it instead would mean
    arguing with the antialiasing of whatever text happens to be nearby.
    """
    x0, y0, x1, y1 = moment.box(claim["role"])
    band = claim.get("band", 8)
    without = moment.without(claim["role"])

    for y in range(max(0, y0 - band), min(moment.picture.h, y1 + band)):
        for x in range(max(0, x0 - band), min(moment.picture.w, x1 + band)):
            if x0 <= x < x1 and y0 <= y < y1:
                continue

            here, there = moment.picture.at(x, y), without.at(x, y)
            if here != there:
                away = max(x0 - x, x - x1 + 1, y0 - y, y - y1 + 1)
                return (f"{claim['role']} paints {here} at {x},{y}, "
                        f"{away}px outside itself, where {there} belongs")

    return None


def check_corners(moment, claim):
    """A role's own corners are its own ground, which is to say square."""
    x0, y0, x1, y1 = moment.box(claim["role"])
    ground = moment.token(claim["is"])
    inset = claim.get("inset", 1)

    corners = [
        (x0 + inset, y0 + inset), (x1 - 1 - inset, y0 + inset),
        (x0 + inset, y1 - 1 - inset), (x1 - 1 - inset, y1 - 1 - inset),
    ]

    wrong = [at for at in corners if moment.picture.at(*at) != ground]
    if wrong:
        found = [moment.picture.at(*at) for at in wrong]
        return f"{len(wrong)} corners of {claim['role']} are {found}, not --{claim['is']}"

    return None


def check_fill(moment, claim):
    """A patch of a role is one flat colour, and the right one."""
    x0, y0, x1, y1 = moment.box(claim["role"])
    inset = claim.get("inset", 2)
    x0, y0, x1, y1 = patch(claim.get("patch", "all"), x0, y0, x1, y1)

    seen = moment.picture.colours(x0 + inset, y0 + inset, x1 - inset, y1 - inset)
    wanted = moment.token(claim["is"])

    if seen != {wanted}:
        return f"{claim['role']} is {sorted(seen)[:3]}, not --{claim['is']} {wanted}"

    return None


def check_ink(moment, claim):
    """Two roles are painted at the same strength.

    What is being asked is whether something has been dimmed, and the answer
    that does not depend on what a host drew inside a window is the darkest
    pixel in it: the ink. Dimming lifts it and nothing else does.
    """
    a = moment.box(claim["role"])
    b = moment.box(claim["same-as"])

    ink = moment.picture.darkest(*a)
    other = moment.picture.darkest(*b)

    if ink != other:
        return f"{claim['role']} inks at {ink} and {claim['same-as']} at {other}"

    return None


def patch(name, x0, y0, x1, y1):
    """A part of a role's box, for claims that must miss whatever is written."""
    if name == "all":
        return x0, y0, x1, y1
    if name == "right":
        return x0 + (x1 - x0) * 2 // 3, y0, x1, y1
    if name == "bottom":
        return x0, y0 + (y1 - y0) * 2 // 3, x1, y1

    raise Claimless(f"there is no patch called {name!r}")


CHECKS = {
    "edge": check_edge,
    "outside": check_outside,
    "corners": check_corners,
    "fill": check_fill,
    "ink": check_ink,
}


# --- Running them -------------------------------------------------------


def run(claims, break_with=None):
    """Every claim, grouped so that one gesture is played once."""
    results = []

    for gesture in dict.fromkeys(claim["gesture"] for claim in claims):
        here = [claim for claim in claims if claim["gesture"] == gesture]
        said, picture = visit(gesture, break_with)
        moment = Moment(said, picture, gesture, break_with)

        for claim in here:
            try:
                wrong = CHECKS[claim["check"]](moment, claim)
            except Claimless as stopped:
                wrong = str(stopped)
            except KeyError as missing:
                wrong = f"no check called {missing}"

            results.append((claim, wrong))

    return results


def report(results):
    bad = 0
    for claim, wrong in results:
        if wrong:
            bad += 1
            print(f"  FAIL  {claim['name']}\n        {wrong}\n        {claim['says']}")
        else:
            print(f"  ok    {claim['name']}")

    return bad


def main():
    claims = tomllib.loads(CLAIMS.read_text())["claim"]

    only = None
    if "--only" in sys.argv:
        only = sys.argv[sys.argv.index("--only") + 1]
        claims = [claim for claim in claims if only in claim["name"]]

    if not claims:
        raise SystemExit(f"no claim is named {only!r}")

    if "--self-check" not in sys.argv:
        print(f"{len(claims)} claims")
        raise SystemExit(1 if report(run(claims)) else 0)

    # A claim nobody can break is a claim that proves nothing, so each one
    # carries the one-line change it is there to catch and is run against it.
    print(f"self-check: {len(claims)} claims, each broken on purpose")
    bad = 0

    for claim in claims:
        if "broken-by" not in claim:
            print(f"  FAIL  {claim['name']}\n        nothing is named that would break it")
            bad += 1
            continue

        [(_, wrong)] = run([claim], break_with=claim["broken-by"])
        if wrong:
            print(f"  ok    {claim['name']}  (caught: {wrong})")
        else:
            print(f"  FAIL  {claim['name']}\n        passed with `{claim['broken-by']}` applied")
            bad += 1

    raise SystemExit(1 if bad else 0)


if __name__ == "__main__":
    main()
