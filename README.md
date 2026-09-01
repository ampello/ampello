<div align="center">

<img src="src-tauri/icons/128x128.png" width="96" height="96" alt="">

# Ampello

**Universal text expansion for Windows.**
You tell Ampello *when I type this, put this there* — and nothing else.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-6135e8.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-6135e8.svg)](#requirements)
[![Release](https://img.shields.io/github/v/release/ampello/ampello?color=6135e8)](https://github.com/ampello/ampello/releases)
[![Build](https://github.com/ampello/ampello/actions/workflows/ci.yml/badge.svg)](https://github.com/ampello/ampello/actions/workflows/ci.yml)

</div>

---

Type a short trigger anywhere on your machine — a browser, an editor, a chat
box — and Ampello replaces it with whatever you said it stands for: a
signature, a paragraph, a page of code, a set of files, or all of those at once.

```
TRIGGER  →  CONTENT
:email      Hello,
            Thank you for reaching out.
            …

:report     See the attached.
            📎 contract.pdf  📎 figures.xlsx  📎 chart.png
```

A snippet is a trigger, a body of text, and any files that go with it. Ampello
never asks what kind of text it is, or what kind of file. One character or
fifty thousand: identical at the data-model level. A PNG, a PDF and a `.docx`:
likewise.

## Install

Download the installer from [Releases](https://github.com/ampello/ampello/releases)
and run it. The installer asks whether to install:

- **Just for me** — no administrator rights needed.
- **For all users on this machine** — requires administrator rights.

Either way, each Windows account gets its own private snippet library by
default. See [Sharing a library](#sharing-a-library-between-accounts) to change
that deliberately.

Builds are provided for **x64** and **ARM64**. The installers are not
code-signed, so SmartScreen will warn on first run — choose *More info* →
*Run anyway*, or build from source.

## What it does

**Triggers fire when you finish them.** `:email` alone does nothing.
`:email` followed by a space, full stop, Tab or Enter expands — Ampello
swallows that key, erases the trigger, inserts the content, then sends the key
back, so `:email.` keeps its full stop. Word boundaries are respected by
default, so `something:sig` stays as typed.

**Snippets can carry files.** Any files. They are handed to whatever
application you are typing in, which a browser turns into an upload and a chat
prompt shows as an attachment. You set the order; there is a strict mode for
applications that would otherwise reshuffle them.

**A clipboard shortcut for when pasting fails.** A second global combination
inserts whatever is already on your clipboard, optionally as individual
keystrokes rather than a paste. Ctrl+V itself is left untouched.

**Escape stops an insertion.** A long snippet takes real time to deliver;
pressing Escape halts it where it is.

**Everything stays on your machine.** One SQLite file, no account, no sync, no
telemetry. The log records what Ampello did, never what you typed.

## Sharing a library between accounts

By default every Windows account has its own library, private to that account.

On a shared machine you can point two or more accounts at the same folder so
they use the same snippets and attachments:

1. **Settings → Data → Location → Share…**
2. Choose a folder every account can reach — `C:\Users\Public\Ampello` is
   offered by default because Windows makes it writable by all accounts.
3. Restart Ampello.
4. Repeat on each account that should join.

Nothing is shared unless an account is pointed at the folder itself. Switching
back is *Use personal*, and the shared library is left untouched.

Two things to know:

- **Snippets are not copied between libraries.** Use Export and Import to move
  them.
- **A running instance does not see another account's edits live.** Changes
  made while you are signed in appear the next time Ampello starts. This
  matters only if two accounts are signed in at once via fast user switching.
- **Keep it on a local disk.** SQLite's write-ahead log does not work reliably
  over a network share.

## Where your data lives

| | |
| --- | --- |
| Personal library | `%APPDATA%\com.yohann.ampello\ampello.db` |
| Attachments | `attachments\` beside the database |
| Shared library | wherever you point it |
| Log | shown in Settings → Data → *Where things are kept* |

Attachments are stored by the SHA-256 of their contents, so the same file on
ten snippets is stored once. Backups are readable YAML you can diff and
hand-edit; a library with attachments exports as a `.ampellozip` archive with
that same document at its root and the files beside it.

## Requirements

Windows 10 (up-to-date) or Windows 11. The WebView2 runtime is preinstalled on
both.

## Build from source

| Tool | Notes |
| --- | --- |
| Node.js 20+ | `node --version` |
| Rust (stable, `x86_64-pc-windows-msvc`) | `rustc --version` |
| Visual Studio Build Tools, "Desktop development with C++" | needed by Tauri and SQLite's bundled C source |

```bash
npm install
npm run tauri dev
```

The first build compiles the Tauri crates and takes a few minutes; afterwards
it is fast.

```bash
npm run typecheck                                             # TypeScript
npm run build                                                 # typecheck + bundle
cargo test --manifest-path src-tauri/crates/ampello-core/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri build                                           # NSIS installer
```

> **Changing the app icon?** `tauri-build` does not watch the icon files, so
> run `cargo clean -p ampello` first or the build will silently keep the old
> one.

## Project layout

```
src/                        React + TypeScript interface
  index.css                 the design token system, both themes
  lib/                      types and the single typed IPC boundary
  stores/                   Zustand: settings, ui, data
  views/                    dashboard, snippets, settings, editor

src-tauri/
  src/                      window, tray, commands, Windows input
    library.rs              personal vs shared library resolution
    input/win/              the keyboard hook, injector and clipboard
  crates/ampello-core/      no platform dependency whatsoever
    attachments.rs          content-addressed store for attached files
    engine/                 boundary rules, rolling buffer, matcher
    db/                     schema, migrations, CRUD, settings
```

`ampello-core` has no Tauri dependency. It compiles and is tested on any
platform, which is what keeps a future macOS or Linux port from being a rewrite
and lets the storage and matching logic be tested without a window.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how a keystroke becomes an
expansion, and [docs/TESTING.md](docs/TESTING.md) for the manual test pass.

## Known limits

All of these are Windows' rules rather than Ampello's, and none can be detected
in advance.

- **Elevated windows.** A normal-privilege process cannot see keystrokes going
  to a window running as administrator, or send input to one. Ampello has to be
  elevated too.
- **Secure input fields.** UAC, the lock screen and some password managers are
  deliberately invisible to hooks. That is a feature.
- **Anti-cheat software and some games** read input below the hook layer, or
  refuse synthetic input outright.
- **Applications that ignore Ctrl+V**, such as older console hosts. Use
  Settings → Expansion → Insertion method → Type.
- **Attachments in a plain text box.** A field with no paste handler for files
  takes nothing at all, silently.

Tested against Notepad, Notepad++, Chrome, VS Code, Windows Terminal, Discord,
Word, Slack and Explorer's rename field.

## Contributing

Bug reports and pull requests are welcome. See
[CONTRIBUTING.md](CONTRIBUTING.md).

## Security

Ampello reads every keystroke on the machine, so please report security issues
privately rather than in a public issue. See [SECURITY.md](SECURITY.md).

## License

[GNU General Public License v3.0 or later](LICENSE).

Ampello is free software: you may redistribute and modify it under the terms of
the GPL. It comes with **no warranty**. If you distribute a modified version,
you must release your changes under the same license.
