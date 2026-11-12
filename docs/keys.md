# Key bindings

What has been decided about binding keys to commands, before any of it is
built. Every command in the program can be run from a key, the bindings are
entirely configurable, and what a key does depends on which window has focus.

## Its own machine

Bindings are a state machine of their own, `keys-state`, beside `dock-state`
rather than inside it. It knows nothing about windows or rects; the dock knows
nothing about keys. The two meet only in the runtime, which forwards what one
emits to the other and decides nothing.

Parsing the config is in the same crate, in its own file, apart from the
machine. Parsing is pure, so it can live in the library, and every host then
gets the same format and the same errors.

## What goes in and what comes out

In:

- **A key going down, and a key coming up.** The raw presses and nothing
  more: the key by what it means rather than where it sits, and the modifiers
  held. Sequences, timeouts and everything else are built from these inside
  the machine. The runtime classifies nothing.
- **A tick**, carrying the time. The machine keeps it and compares against it.
- **The contexts in force**, as a list of names — see below.
- **A new keymap**, at any time.

Out:

- **Run this command.**
- **This keymap is wrong**, and where.

## A binding names a command

A binding is a sequence of one or more chords, and it names a command: the
same commands the launcher lists, so anything that can be bound can be found
by name, and the launcher can show the keys beside it. A binding passes no
argument to the command. If that is ever needed, it is decided along with
commands that take parameters, one case at a time.

The primary modifier is Cmd on macOS and Ctrl elsewhere, so one config serves
both.

## Commands have names, and live in modules

Every command has a proper name, `Module::Command`: `Dock::FocusNext`,
`Launcher::Open`, `Editor::Restart`. The module is the part of the program the
command belongs to and the command is what it does, as in Zed. The library
defines its own modules (`Dock`, `Launcher`) and the host defines the rest. The
name is what a binding refers to, so it is stable: renaming a command is a
change to every config that binds it.

**A module is only a namespace**, and which module a command goes in is
decided by whoever defines it. It says nothing about where the command can
run: `Timeline::PlayPause` is not tied to a timeline having focus. Where a key
works is decided by contexts alone.

**Each command also has a display name**, written by whoever defines it, and
that is what the launcher lists: `Dock::CloseWindow` is shown as *Close
window*. The name is for configs and the display name is for people.

## Sequences, and the timeout

`ctrl-k ctrl-c` is two chords. While a sequence is part way through, the
machine holds what has been typed so far.

When one binding is the beginning of another — `g` and `g g` — the shorter one
runs if nothing continues it within **3 seconds**, a constant for now. A key
that continues no binding ends the sequence. Nothing is drawn while a sequence
is pending.

## Contexts

The host gives each window a list of **contexts**: always its kind
(`timeline`, `canvas`), and `text` while an input in it is taking text. The
contexts in force are those of the focused window. A key is looked up in the
maps of those contexts first and the global map last, and **the most specific
binding wins**. A context can also unbind a key that the global map binds.

Typing is then not a special case. The `text` context unbinds every key
without a modifier, so `g` reaches the input rather than running a command.
Vi-style modes would be one more context the day they are wanted, and they are
not wanted yet.

Contexts are plain names. Conditions like Zed's `editor && !text` wait until a
real case needs them.

## Focus

One window has focus at a time. Focus is the dock's, because it is about
windows: a click or a command moves it, and the dock says where it went. The
runtime passes that on to `keys-state` as the contexts of the window that now
has it.

## The file

TOML, with a table per context. It will mostly be edited by an assistant on
the user's behalf, so it is written to be edited safely without being read
whole: one binding per line, comments allowed, and every error names the line
it is on. Rewriting one binding in place, with `toml_edit`, leaves the comments
around it untouched.

```toml
[global]
"primary-p"         = "Launcher::Open"
"ctrl-k ctrl-c"     = "Editor::Comment"

[timeline]
"space"             = "Timeline::PlayPause"
"ctrl-k ctrl-c"     = false

[text]
unbind-unmodified   = true
"escape"            = "Text::Leave"
```

A context unbinds every key without a modifier with one flag,
`unbind-unmodified`, and unbinds a single key by binding it to `false`. The
flag cannot be mistaken for a binding, because `unbind` is not a modifier. A
binding in the same context still wins over the flag, so `text` can unbind
plain keys and keep `escape` for itself.
