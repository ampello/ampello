# Testing Ampello

Most of Ampello is covered by automated tests: 83 in `ampello-core`, run with

```powershell
cargo test --manifest-path src-tauri/crates/ampello-core/Cargo.toml
```

They cover trigger matching, word boundaries, Unicode, the storage layer,
backups and a set of performance floors. What they cannot cover is the part
that only exists once Windows is involved: the keyboard hook, text injection,
the clipboard, and how real applications behave. That is what this list is for.

Work through it after a change to anything under `src-tauri/src/input/`.

---

## The one that matters

Ampello running in the background. Open Notepad. Type `:hello` then a space.
The trigger disappears; the snippet appears.

If that works, everything below is refinement.

---

## Expansion

- [ ] `:hello` + **space** expands.
- [ ] `:hello.` expands and **keeps the full stop**.
- [ ] `:hello` + **Enter** expands, and in Discord or Slack the message is then sent.
- [ ] `:hello` + **Tab** expands.
- [ ] `something:hello ` does **not** expand.
- [ ] After switching Trigger position to *Anywhere*, it does.
- [ ] `:hell`, backspace, `o`, space still expands.
- [ ] Type `:hel`, click elsewhere, type `lo ` does **not** expand.
- [ ] Type `:hel`, press ←, type `lo ` does **not** expand.
- [ ] Two triggers where one ends with the other (`sig` and `mysig`): typing `mysig ` expands the longer one.
- [ ] A disabled snippet does not expand; re-enabling it takes effect without restarting.
- [ ] The global switch (Settings or tray) stops expansion immediately.

## The list

- [ ] Click a snippet and press **Delete**: the same confirmation appears as
      from its menu, Enter confirms it and Escape cancels.
- [ ] Type in the search box and press **Delete**: it deletes a character and
      nothing else.
- [ ] Arrow up and down from the search box moves the selection; Enter opens it.
- [ ] Deleting the selected snippet leaves nothing selected and does not open
      the editor.
- [ ] Every empty state (no favourites, empty collection, no search results, a
      brand new library) is text only: no icon above it and no button in it.
- [ ] A row shows its trigger and the start of its content. There is no title
      anywhere: not in the editor, not in the list, not on Home.
- [ ] Import a backup exported by an older build (one with `title:` lines).
      It imports cleanly and the titles are simply ignored.
- [ ] Open a collection, press New Snippet in the sidebar: the Collection field
      is already set to that collection, and the field can still be changed.
- [ ] Do the same from Home, from Favorites, and from All Snippets: the
      Collection field is None.
- [ ] Save one of those and it appears in the collection it was started from.

## Content

- [ ] Multi-line content arrives with its line breaks intact.
- [ ] Indentation is preserved. Expand a snippet of tab-indented code into VS Code and check nothing was re-indented.
- [ ] Unicode and emoji arrive intact.
- [ ] A very large snippet (50,000+ characters) expands, and in reasonable time.
- [ ] A one-character snippet expands.
- [ ] An empty snippet removes the trigger and inserts nothing.

## Stopping an insertion

- [ ] Expand a very long snippet into an editor and press **Escape** while it
      is still going in. It stops where it is, nothing more is typed, and the
      editor is still in front, and the Escape must not also close a dialog or
      leave the field.
- [ ] Same again with *How content is inserted* set to **Type**. This is the
      slow path and the one Escape matters most on.
- [ ] Same with **Paste**. There is only a moment to catch it; if you do, the
      clipboard still holds what it held before.
- [ ] After a stopped insertion, press Escape once more in the same editor: it
      must behave normally (close the dialog, leave the field), not be eaten.
- [ ] The trigger's terminating character is **not** re-typed after a stop.
- [ ] Nothing is left stuck down after a stop: Enter, Tab and Backspace all
      behave normally in that application immediately afterwards.
- [ ] At **Careful**, a typed snippet arrives visibly character by character,
      not in blocks. At **Fast** it is a blur but still continuous.
- [ ] In a heavy editor (one running inside a web page, or Electron), a long
      snippet at **Fast** keeps typing after Escape while **Careful** stops
      promptly. That difference *is* the setting working, because the surplus lives in
      the application's own queue, out of Ampello's reach.
- [ ] The log (Settings → Data → log folder) has an `insertion: N events in
      M ms` line for each long insertion, and no `hook meter unavailable`.

## Clipboard

- [ ] Copy some text. Expand a long snippet. Ctrl+V brings the **original text** back.
- [ ] Copy an **image**. Expand a long snippet. Paste into Paint. The image is still there.
- [ ] Copy **files** in Explorer. Expand a long snippet. Paste into another folder. The files are still there.
- [ ] With *Put your clipboard back* off, the clipboard holds the snippet afterwards, as advertised.
- [ ] With the clipboard empty before an expansion, it is empty afterwards, not holding the snippet.

## The clipboard shortcut

Everything below with **Method → Type**, unless a line says otherwise.

- [ ] Copy a paragraph. Ctrl Shift V into Notepad types it out at the typing speed set above, rather than pasting it in one go.
- [ ] Ctrl+V still pastes normally in the same field. The shortcut adds a combination, it does not take one away.
- [ ] Hold the combination down for a second before letting go. The text still arrives intact, and no character comes out as a Ctrl chord.
- [ ] Hold it down long enough for Windows to repeat it. Exactly one insertion happens, not several stacked on top of each other.
- [ ] Copy something long. Press Escape part-way through. It stops, and Escape does not also close the window behind it.
- [ ] Copy an **image**, then Ctrl Shift V. It pastes, because there is nothing to type.
- [ ] Copy **files** in Explorer, then Ctrl Shift V into another folder. The files are pasted.
- [ ] The clipboard still holds the same thing after every one of the above. This shortcut only reads it.
- [ ] With **Method → Paste**, Ctrl Shift V behaves as a plain Ctrl+V, instantly.
- [ ] Switch **Enabled** off. Ctrl Shift V goes back to whatever the application does with it (in Chrome, paste without formatting).
- [ ] Pause expansion from the tray. Ctrl Shift V goes back to the application in the same way. Resume, and Ampello takes it again.
- [ ] Rebind it to another combination. The old one is released, the new one works.
- [ ] Rebind it to the **global shortcut's** combination. Settings says so and refuses to register it.
- [ ] Rebind it to something another application already owns. Settings says that, rather than failing quietly.
- [ ] Ctrl Shift V inside Ampello's own editor inserts the clipboard there too. It is a global shortcut, so it is not supposed to make an exception for Ampello.

## Applications

Expand a short snippet and a multi-line one in each:

- [ ] Notepad
- [ ] Notepad++ / VS Code
- [ ] Chrome, an address bar and a text field
- [ ] Discord / Slack
- [ ] Windows Terminal (if pasting misbehaves, try *How content is inserted → Type*)
- [ ] Word
- [ ] Explorer's rename field
- [ ] An application running **as administrator**. Expansion is expected **not** to work unless Ampello is elevated too. Confirm it fails cleanly rather than eating the keystroke.

## Background

- [ ] Close the window. Ampello stays in the tray and keeps expanding.
- [ ] Left-click the tray icon reopens it.
- [ ] Tray → *Expand snippets* and the Settings switch stay in step, both ways.
- [ ] The global shortcut works from another application, and toggles.
- [ ] Starting a second copy of Ampello brings the first forward rather than running two.
- [ ] *Launch at startup*, then reboot: Ampello is in the tray with **no window**.
- [ ] Tray → *Quit Ampello* exits, and expansion stops.

## Scale and endurance

- [ ] Import a backup of several thousand snippets. The list still scrolls smoothly and search still feels instant.
- [ ] Leave Ampello running for a full working day. At the end: expansion still works, and Task Manager shows idle CPU near zero and memory that has not crept.
- [ ] Settings → Expansion: the keystroke count is climbing. If it is stuck at zero while you type, Windows dropped the hook. Press **Restart** and it should recover.

## Recovery

- [ ] Quit Ampello, corrupt `ampello.db` (open it in a text editor and scribble on it), start Ampello. It should start with an empty library, tell you, and leave the damaged file on disk next to it.
- [ ] Import a backup. The library comes back.

---

## Definition of done

A Windows user can install Ampello, create a trigger and arbitrary-length
content, save it, close the window, keep Ampello running in the tray, open
another application, type the trigger, and have it replaced, with multi-line,
long-form and Unicode content, indentation and whitespace intact. They can edit
it later, search, favourite, organise into collections, disable one snippet or
all of them, use the tray, import and export, switch between Light, Dark and
System, and drive the whole thing from the keyboard.

A one-word replacement and a fifty-thousand-character document are both just
content, and Ampello treats them identically.
