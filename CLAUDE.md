# CLAUDE.md

- Be concise in your responses.
- If a question can be answered by looking at the code, look at the code
  instead of asking the user.
- **Never ask the user anything through a form.** No option pickers, no
  multiple choice, no `AskUserQuestion`. When you have a question, write it in
  plaintext in your reply and let the user answer in their own words.
- **Never leave anything running.** Start a server, a browser or a watcher to
  check your own work if you need one, and stop it before you answer. The user
  runs the app themselves, and a port already taken by a process they did not
  start is a confusing thing to walk into.
- Keep comments sparse. A comment explains why something is the way it is; it
  does not restate the line below it. Never write a comment that is a row of
  dashes.
- **A comment is short.** One sentence above a function, a few words above a
  constant, a few words after a field. Reasoning that needs a paragraph goes
  in `docs/`, where it can be read and revised on its own.

## What this is

**weaver** is a tiled dock: a library for arranging windows on a surface,
where a window can be dragged somewhere else, dropped, and resized, where
windows that share a slot wear a tab each, and where a window you put
somewhere stays there until somebody moves it again.

It is a library and not a program. The host supplies the windows and draws
what is inside them; the dock decides only where the rectangles go. It will be
used to build several programs, and `hl`'s browser UI is meant to end up on
top of it.

**Scope, as decided so far.** Each of these was a choice and each can be
revisited, but none of them is a gap to be helpfully filled in:

- **Tiled, never overlapping.** No z-order, no floating windows. Windows
  never overlap each other; what is drawn over them is the ghost under the
  cursor mid-drag and the notifications in the bottom right corner.
- **No tabs yet.** A slot holds one window. Tabs are wanted later, which is why
  nothing claims the centre of a window as a drop target.
- **No components.** No buttons, rows, inputs, palettes. The library draws a
  title bar, a gap, a ghost, a preview and a notification card with its
  actions, and that is the whole list.
- **The web first.** Native comes later, and GPUI is the intended backend for
  it — which is why nothing in the machine may name a browser type.
- **The theme is not part of the library.** It is a role set a host can
  replace. What is drawn now is borrowed from `hl`: black on white, black as
  the accent.

## The workspace

One crate per concern, named `<group>-<thing>`. Dependencies only ever point
down this list.

```
dock-state   the dock: the tree, the solve, the gestures. No IO, no browser
dock-web     the dock in a browser: a frame loop, one pointer listener
web-main     a page with four windows in it, to drag around
```

`dock-state` is the whole library. `dock-web` decides nothing — it forwards
pointer positions and moves elements to the rectangles the frame names, and
`dock-gpui` will be the same file against a different screen.

`crates/dock-web/dock.css` is everything the library draws. The sketches wear
that file rather than a copy, so a change to one is a change to both. The
theme it names roles from is `sketches/design/theme.css`, which is the host's
to replace.

## The design

**Read `sketches/dock/index.html` before changing anything about how the dock
behaves.** Every state is drawn there with the argument for it, and a decision
that has not been taken is on the board as a dashed note. A state is drawn
once, on the board, and nowhere else — a screen argued about twice is a screen
that drifts.

## The state machine

**The dock is a state machine, and the rules are not negotiable.** They are in
the `state-machines` skill (`~/.claude/skills/state-machines/`). Read it before
touching the machine; what follows is only what is specific to this library.

- **Two entry points.** `handle_events(&[Event])` applies a whole batch, then
  `emit_actions(&mut Vec<Action>)` answers it once. Handling an event never
  emits. A browser delivers a burst of pointer moves per frame, so the batch is
  a frame and the dock lays out once per frame by construction.
- **Nothing derived is stored.** No rect is cached, and neither is the drop
  target: it is a function of where the pointer is, computed on the way out, so
  the preview the user sees and the slot the drop commits are the same
  computation and cannot disagree.
- **Hit testing is in the machine**, because the machine owns the rects. The
  runtime forwards raw pointer positions and classifies nothing.
- **No clock.** The drag threshold is a distance. The day something needs time —
  a double click, a press and hold — it arrives as an event, never as a timer
  in the runtime.
- **The alphabet is as small as the program can prove it needs.** Add the
  obvious events and actions; leave out anything you are unsure of and add it
  when the thing that needs it is in front of you.

## Tests

**Read `references/testing.md` in the `state-machines` skill before writing
one.** What follows is only what is specific here.

Every harness test lives in `crates/dock-state/tests/`, split by the flow it
is about — `moving`, `resizing`, `tabs`, `opening`, `settling` — each opening
with that flow's whole story and following it with the corners of it. The
dock a test starts from and the questions it asks of a finished frame are in
`tests/fixture/`; **nothing in there builds an event**, because an event
belongs on the `<` line that sends it, written out in full where it can be
read.

**One `eval!` per test.** The dock has no actions yet, so every batch is
silent and a test is one block of `<` lines followed by plain assertions on
the frame. A claim that needs to be made part way through a gesture is its
own test that stops there — what happens after the drop is the next test's
claim.

A coordinate in a test is a point on a measured layout, so the numbers are
written out rather than computed: 800 by 600 with a 1px gap makes two windows
399.5 wide, and when a metric changes those numbers change with it. That is
the diff doing its job.

## Watching it move

`tools/trace.sh` makes the page perform a gesture and writes down where every
slot, tab and gap was **on each animation frame**, and `tools/read-trace.py`
says what moved, when, and along what curve. Use it before arguing about how
something feels: a settle is almost impossible to judge a frame at a time by
eye, and the questions are usually about timing.

```
cd crates/web-main && trunk serve
tools/trace.sh "down:1120,13|move:1000,100|move:40,200|up:40,200" > /tmp/t.json
python3 tools/read-trace.py /tmp/t.json
```

Two things to know before trusting a headless browser here. A screenshot taken
under `--virtual-time-budget` cannot be pinned to a moment, because virtual
time stops while the wasm is fetched and the budget is spent before the page
is alive. And under virtual time `requestAnimationFrame` barely runs, so the
frame loop this library is built on hardly turns — which is why the trace runs
the page in real time and has it post what it saw.

Aim with measured numbers rather than assumed ones: the dock is not as tall as
the window, and a drop a few pixels out lands somewhere else entirely. Run a
gesture that does nothing (`down:5,5|up:5,5`) to read the layout first.

## Checking what it looks like

`tools/probe.py` checks the dock against `tools/claims.toml`, which is the
list of things it has to look like — one claim per line of the argument the
board makes. A claim plays a gesture, holds there, and asks two questions of
that moment: where the things the dock draws are, which the page answers, and
what colour each pixel is, which a still answers. Nothing in it reads the
stylesheet, so the same claims can be put to a backend that is not a browser.

```
cd crates/web-main && trunk serve
tools/probe.py                 # every claim
tools/probe.py --only shadow   # the ones whose name matches
tools/probe.py --self-check    # break each claim on purpose, and watch it fail
```

**A claim carries the change that has to break it.** `broken-by` is one line
of CSS, and `--self-check` applies it and fails the claim that then passes. A
claim nobody can break proves nothing, and that is the one thing about a test
of a picture that cannot be checked by looking at the picture.

Claims are about flat fills and one pixel lines, never about text: the shape
of a glyph is the font's business and not the dock's. What is about rects and
order belongs in `dock-state`'s tests instead, where no browser is needed.

## Commits

A commit is small and does one thing. Its description is at most three
sentences and one paragraph — no bullet lists and no headings. The
`Co-Authored-By` trailer is welcome.
