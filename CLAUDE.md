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

- **Tiled, never overlapping.** No z-order, no floating windows. The one thing
  ever drawn over a window is the ghost under the cursor mid-drag.
- **No tabs yet.** A slot holds one window. Tabs are wanted later, which is why
  nothing claims the centre of a window as a drop target.
- **No components.** No buttons, rows, inputs, palettes. The library draws a
  title bar, a gap, a ghost and a preview, and that is the whole list.
- **The web first.** Native comes later, and GPUI is the intended backend for
  it — which is why nothing in the machine may name a browser type.
- **The theme is not part of the library.** It is a role set a host can
  replace. What is drawn now is borrowed from `hl`: black on white, black as
  the accent.

## The workspace

One crate per concern, named `<group>-<thing>`. Dependencies only ever point
down this list.

```
util-eval    the eval! macro every state machine test is written through
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

**Harness tests live in the crate's `tests/` directory, never beside the
machine.** A test is a slice of events in and a list of action patterns out,
driven by `eval!`. Unit tests — arithmetic, a wedge, a solve — stay inline.

## Building

Every cargo command covers the whole workspace. Never narrow one with `-p`:
cargo resolves features from the crates it was asked to build, so `-p` builds a
second copy of half the tree.

```
cargo clippy --workspace --all-targets
cargo test --workspace
```

The browser build is the exception, and only because cargo cannot build a
workspace for two targets at once: trunk builds `web-main` alone, for
`wasm32-unknown-unknown`.

```
cd crates/web-main && trunk serve      # http://127.0.0.1:7150
```

## Commits

A commit is small and does one thing. Its description is at most three
sentences and one paragraph — no bullet lists and no headings. The
`Co-Authored-By` trailer is welcome.
