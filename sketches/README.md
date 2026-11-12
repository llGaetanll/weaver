# Sketches

Screens drawn in HTML and CSS before they are built in Rust, so that a state is
argued about once rather than twice. Open any of them in a browser;
`index.html` links them all.

Nothing here runs. Every file is a picture.

## The boards

- `dock/index.html` — every state the dock can be in, each in a frame of its
  own, with the reason it is that way. A state is drawn here and nowhere else.
- `notify/index.html` — notifications in the bottom right corner: the card,
  a stack of them, warnings and errors, and how they come and go. Ids `corner` `card`
  `stack` `kinds` `coming`.

Each sheet carries an id, so from a terminal:

```
grep -n 'id="between"' dock/index.html
```

and in a browser `dock/index.html#between` jumps to it. The ids, in the order
they are drawn: `idle` `resize` `drops` `edge` `between` `outer` `nowhere`
`alone`.

A decision that has not been taken is on the board too, as a dashed note
saying what is missing, so what is undecided is as plain as what is settled.

## The design language

The theme is **not part of the library**. `design/theme.css` is in two parts,
the ramps and the roles (`--bg`, `--panel`, `--border`, `--accent`, `--gap`,
`--tab`), and a board only ever names a role — so the ramps can be replaced
and every picture follows. What is here is borrowed from `hl`: black on white,
black as the accent, red kept for the one thing red is for.

`../crates/dock-web/dock.css` is everything the library itself draws, and
nothing else: a window's title bar, the gap between two windows, and the ghost
and preview that exist only while a window is being moved. There are no
buttons, inputs or rows in it, because the library has no opinion about what
goes inside a window. The boards wear that file rather than a copy of it, so a
change to one is a change to both.
