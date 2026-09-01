# Ampello architecture

How Ampello works underneath, and why it is built the way it is. For
installation and everyday use, see the [README](../README.md).

## How expansion works

```
keyboard ──▶ WH_KEYBOARD_LL hook ──▶ translate ──▶ matcher (ampello-core)
                    │                                    │
                    │  swallows the terminating key       │ match
                    ▼                                     ▼
            target application  ◀── inject ◀────── injector thread
```

Two threads. The hook thread must return from every callback in well under
Windows' hook timeout, so it only translates a key, takes a short-lived lock
and posts a job; reading the snippet, touching the clipboard and sending input
all happen on the injector thread.

**A trigger fires when you finish it.** `:email` alone does nothing; `:email`
followed by a space, a full stop, Tab or Enter expands. Ampello swallows that
terminating keystroke, erases the trigger, inserts the content, and then sends
the keystroke, so `:email.` keeps its full stop and `:email` + Enter still
sends the message.

**Word boundaries by default.** `hello :sig` expands, `something:sig` does not.
Settings → Expansion → Trigger position → Anywhere removes that protection.

**Two ways in.** Short single-line snippets are typed as individual keystrokes
and never touch the clipboard. Longer, multi-line or indented ones are pasted,
which means borrowing the clipboard: Ampello copies out every clipboard format
backed by global memory first (text, HTML, RTF, DIB images, copied file lists)
and puts them back afterwards. If it finds a format it *cannot* copy, and the
snippet is short enough to type, it types instead rather than destroying
something it could not restore. You can force either route in Settings.

**Tabs and newlines need opposite treatment.** A newline is sent as a real
Return key. Sent as a Unicode carriage return it is ignored by anything built
on a web view, and the snippet arrives as one long line. A tab is sent as a
Unicode character and never as the Tab key, because Tab moves focus to the
next control instead of indenting, which is how a snippet of indented code
ends up typing itself into a toolbar. The cost is that an application ignoring
character-level tabs will drop the indentation. Auto sidesteps the question by
pasting anything indented; **Type never touches the clipboard at all**,
because that is the one thing it exists for.

**A snippet can carry files.** Any files - an image, a PDF, a .docx, an
archive. Nothing in Ampello classifies them: the only place the kind of file
matters at all is that an image can additionally go over as a picture, embedded
in the message, where a document can only ever be attached. There is no
clipboard format that means *insert this PDF here*.

The files are handed to the application as a drop, which is what a browser turns
into an upload and what a chat prompt shows as an attachment. Applications that
have no paste handler for files - a plain text box, Notepad - take nothing at
all, silently, because there is no way to ask one in advance whether it will.

**Text and files are separate pastes.** A single Ctrl+V delivers one payload: an
application offered both takes whichever it prefers and drops the other. So they
are sequenced, and the clipboard is borrowed once around the whole run rather
than once per file.

**Order is the user's, up to a point.** The files go over as one list, in the
order set in the editor, and every application that respects that list order
shows them that way. One that uploads in parallel will show whichever finishes
first, so a small icon queued third can land ahead of a large screenshot queued
first. *Keep this exact order* hands them over one at a time and waits for each
instead - slower, and the only way to make the order actually stick. How long to
wait is Settings → Expansion → Attachment delay, because there is no way to ask
an application whether it has finished taking a file.

**Attached files live beside the database**, in an `attachments` directory,
addressed by the SHA-256 of their contents so the same file on ten snippets is
stored once. The original filename is kept, because delivery hands the
application a path and the application shows the user its last component. A
library that has any of them exports as a `.ampellozip` archive: the same readable
backup document at the root, the files beside it. A library with none exports as
the plain YAML or JSON it always did.

**An insertion can be stopped.** A long snippet takes real time to deliver;
pressing **Escape** while it is going in stops it where it is.

Two things had to be true for that to work reliably. The first is that Escape
cannot be detected through the keyboard hook: while an insertion is in flight
there are thousands of injected events queued ahead of the user's keystroke,
and a hook sees them strictly in order, so by the time it heard about Escape
the insertion would already have finished. A thread of its own watches the
physical state of the key every few milliseconds instead, and the keystroke is
swallowed when it finally arrives so that stopping an insertion does not also
close whatever is in front of it.

The second is that Ampello must not run away from Windows. `SendInput` returns
as soon as Windows *accepts* an event, not when anything has been done with it,
so typing flat out just builds a queue inside Windows, and every keyboard event
on the machine, Ampello's and the user's alike, goes through that one queue in
order. Thousands of events ahead means the user's Escape is stuck behind all of
them: physically pressed, and not noticed until the snippet has finished going
in. That is why it used to get less reliable the longer an insertion ran.

Ampello's own events come back through its own keyboard hook, so counting them
tells it exactly how many of its keystrokes are still inside Windows, and it
never gets more than about sixty characters ahead of that. What it cannot see
is the queue *inside the receiving application*, and there is no API that will
tell it. A plain text box takes everything Windows can send, an editor running
inside a web page does not. That is what **Typing speed** in Settings is for:
if a long insertion carries on typing after you have stopped it, the surplus is
sitting in the application's own queue and a slower speed is the only thing
that prevents it.

Typing is done one character at a time, one `SendInput` each, at the chosen
rate, rather than in batches. Batching was faster to write and marginally
cheaper to run, and it made typing look like a series of small pastes, because
that is what it was. Fast is around 250 characters a second, Balanced around
150, and Careful around 60, which is roughly a quick typist.

What Ampello can still do at that point is refuse to make it worse. Cancelling
raises a flag that its keyboard hook reads, and every one of Ampello's own
keystrokes that has not yet reached the application is thrown away there rather
than passed on, the last point in the journey where it can still be caught.

Ampello also stops by itself the moment the caret leaves where it started: a
different window, or a different field within the same window. Without that, a
focus change part-way through would send the rest of a snippet somewhere it
was never meant for, where letters are menu accelerators and Return presses
whatever button is in front.

**Ampello ignores its own output.** Every event it sends carries a marker that
the hook recognises, so backspaces and pastes never feed back into the matcher.

## The clipboard shortcut

Separate from snippets, and off to one side of everything above: a second
global combination, **Ctrl Shift V** by default, that inserts whatever is on
the clipboard. Ctrl+V is left exactly as it is.

It exists for the applications that will not take a paste, or that take one and
reformat it. **Method → Type** sends the clipboard as individual keystrokes at
the same typing speed a snippet uses, so it gets in wherever typing gets in,
and Escape stops it part-way like any other insertion. **Method → Paste** sends
a plain Ctrl+V instead, which is instant and exact. If the clipboard holds an
image or a list of files there is nothing to type, so Ampello pastes either way
rather than swallowing the keystroke and doing nothing.

Two details that are not obvious:

- The combination is still held down when it fires, and Windows repeats a held
  modifier as a stream of fresh key-down events. Ampello waits for you to let go
  before it sends anything, up to about a second, otherwise the tail of a long
  insertion arrives as Ctrl chords.
- Switching the shortcut off, or pausing expansion from the tray, unregisters
  it rather than registering it and doing nothing. A shortcut Ampello has claimed
  and ignores would be worse than one it never took: the application in front
  gets the combination back.

It cannot be the same combination as the global shortcut, and if another
application has already claimed it Ampello says so in Settings instead of
failing quietly.

## Where it works, and where it will not

Tested targets: Notepad, Notepad++, Chrome, VS Code, Windows Terminal, Discord,
Word, Explorer's rename field, Slack.

Known limits, all of them Windows' rules rather than Ampello's:

- **Elevated windows.** A normal-privilege process cannot see keystrokes going
  to a window running as administrator, and cannot send input to one. If you
  need expansion inside an elevated application, Ampello has to be elevated too.
- **Some games and anti-cheat software** read input below the hook layer, or
  block synthetic input entirely.
- **Applications that ignore Ctrl+V**, such as older console hosts, need
  Settings → Expansion → Insertion method → Type. The clipboard shortcut, set
  to Type, is the same answer for ordinary copied text.
- **Secure input fields** (UAC, the Windows lock screen, some password
  managers) are deliberately invisible to hooks. That is a feature.
- **Emoji and other astral-plane characters inside a trigger** are counted in
  UTF-16 code units when erasing, which is what most text controls use, but a
  few editors delete a whole emoji per backspace and would erase one character
  too many. Triggers made of ordinary text are unaffected.

## Backups

Settings → Backups writes the whole library to one file, and reads one back.

```yaml
# Ampello snippet backup
version: 2
exportedAt: 1787821533838
categories:
  - name: "Work"
    position: 0
snippets:
  - trigger: ":email"
    collection: "Work"
    enabled: true
    favorite: true
    usageCount: 41
    content: |
      Hello,

      Thank you for reaching out.

      Best regards,
      Yohann
```

Snippet bodies come out as block scalars, so a backup is something you can
read, diff and hand-edit. Collections are referenced by name rather than by
id, so a file written on one machine means something on another. Exports are
sorted by trigger, which makes two exports of an unchanged library
byte-identical.

**The file is lossless.** Trigger, body, collection, enabled state,
favourite and usage count all survive a round trip, and the body survives it
byte for byte: tabs, trailing spaces, CRLF, emoji, a megabyte of it. Where
awkward whitespace cannot be written as a readable block scalar, Ampello quotes
it instead. Before writing anything it parses its own output back and compares
it against the library; if the readable form would not survive, it falls back
to the YAML library's own emitter. Legibility is never worth a corrupted
backup.

JSON is offered too, for tools that would rather have it.

**Importing merges** into your library rather than replacing it. A trigger you
already have is left exactly as it is, and Ampello reports what it did: added,
left alone, collections created, and any rows it could not read. One bad entry
never abandons the rest of the file. Hand-written files work as long as each
snippet has a `trigger`; everything else takes a sensible default.

## The window

Ampello draws its own title bar rather than wearing Windows'. There is no
separate strip for it: the row that holds the sidebar's name and the page title
*is* the title bar, so the chrome costs no vertical space at all. Drag it to
move the window, double-click it to maximise.

The three caption buttons keep Windows' own geometry: 46 pixels wide, square
corners, flush into the corner, close turning red, because those are the
controls everyone already knows without looking. They are rendered at the root
of the interface, so they exist even on the loading and failed-to-start
screens, where there would otherwise be no way out of the window.

Closing goes through the same path as the window's own close request, so
"keep running when the window is closed" still decides what closing means.

One consequence worth knowing: hovering the maximise button no longer offers
Windows 11's Snap Layouts, which are drawn by the native frame Ampello replaced.
Win + arrow keys and dragging to a screen edge still work.

## Running in the background

Ampello is only useful while it is running, so closing the window puts it in the
notification area rather than quitting it. The tray icon carries the whole
switchboard:

```
Ampello
  Open Ampello
  ✓ Expand snippets
  ─────────────
  Settings…
  ─────────────
  Quit Ampello
```

Left-click the icon to open Ampello, right-click for the menu. "Expand snippets"
is the same switch as the one in Settings, and flipping either moves the other.

**Ctrl + Shift + Space** brings Ampello forward from anywhere, and pressing it
again puts it away. It is configurable in Settings; if another application has
already claimed the combination, Ampello says so rather than failing silently.

**Launch at startup** registers Ampello with Windows, and **Keep running when
the window is closed** can be turned off if you would rather closing the window
quit it outright.

Only one Ampello runs at a time. Starting a second copy just brings the first
one forward, because two instances would mean two keyboard hooks and every
snippet expanding twice.

## Reliability

Ampello is meant to be left running all day, which makes the failure modes more
interesting than the features.

**A damaged library never blocks startup.** The database is integrity-checked
every time it is opened. A file that cannot be opened, verified or migrated is
*moved aside*, never deleted, and a fresh one takes its place, and Ampello says
so both at startup and permanently in Settings → Storage, with the path to the
old file. Losing a library silently would be much worse than losing it loudly.

**The keyboard hook can be reinstalled without restarting Ampello.** Windows
silently removes a low-level hook whose callback runs too slowly, and the only
symptom is that nothing expands any more. Settings → Expansion shows how many
keystrokes the hook has seen this session, a count and never content, so a dead
hook is visible rather than mysterious, and **Restart** puts a fresh one in
place.

**Nothing polls.** The input engine is driven by hook callbacks, the interface
by events. When Ampello's window is hidden it stops refreshing lists nobody is
looking at, and picks up where it left off when you bring it back.

**A crash in one place cannot take the keyboard with it.** The hook callback
catches panics and falls through to the application, so the worst case is a
missed expansion rather than a swallowed keystroke. It never swallows a key
unless the expansion was successfully queued, and the injector is wrapped the
same way so a failure there cannot leave Ampello permanently deaf.

**There is a log**, at the path shown in Settings → Storage. It records what
Ampello did, never what you typed.

## Performance

Measured on a release build, ten thousand snippets, each with a body of a few
hundred characters:

| | |
| --- | --- |
| Trigger matching | **31 ns** per keystroke |
| Fetching a body to expand | **3.6 µs** |
| Loading every trigger after an edit | **2.9 ms** |
| Listing all 10,000 snippets | **31 ms** |
| Full-content search across all of them | **9 ms** |

The number that matters is the first one. It sits in the keyboard hook, on
every keystroke you type anywhere on the machine, and Windows will remove the
hook if it is slow. A fast typist manages about ten keystrokes a second; 31 ns
is six orders of magnitude of headroom, and it holds because matching is a
handful of hash lookups rather than a scan: triggers are short, so ten
thousand of them collide into only a few distinct lengths.

These are checked by tests, with deliberately loose thresholds: they exist to
catch a change that makes something *categorically* slower, not to police
microseconds.

## Privacy

The keyboard hook runs entirely on this machine. Ampello keeps a rolling buffer
only as long as its longest trigger, throws it away on every click, arrow key,
window change and shortcut, and never writes it anywhere. Nothing you type is
recorded, and nothing leaves the machine.

## Where your data lives

`%APPDATA%\com.yohann.ampello\ampello.db` is a single SQLite file. Local-first:
nothing about your snippets, your keystrokes or your clipboard leaves the
machine. There is no account, no sync and no telemetry.

## Design notes

- **Content is data, never code.** Ampello stores text and replays text. It does
  not execute snippets, expand shell syntax or evaluate anything.
- **No content classification.** No content-type field, no language picker, no
  editor modes. The editor is a universal text editor.
- **Content is preserved exactly.** No trimming, no whitespace normalisation, no
  length limit beyond what SQLite and your machine impose.
