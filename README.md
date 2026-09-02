<div align="center">

<img src="src-tauri/icons/128x128.png" width="88" height="88" alt="Ampello">

# Ampello

**A fast, privacy-respecting text expander for Windows.**

[![License](https://img.shields.io/badge/license-GPLv3-6135e8.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/ampello/ampello?color=6135e8)](https://github.com/ampello/ampello/releases)
[![CI](https://github.com/ampello/ampello/actions/workflows/ci.yml/badge.svg)](https://github.com/ampello/ampello/actions/workflows/ci.yml)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-6135e8.svg)](#requirements)

[Installation](#installation) · [Features](#features) · [Documentation](docs/ARCHITECTURE.md) · [Contributing](CONTRIBUTING.md)

</div>

---

## What is Ampello?

Ampello watches for short abbreviations as you type and replaces them with
longer content. It works in every application on your machine: browsers, code
editors, chat clients, email, terminals.

You define a trigger and the content it stands for. When you finish typing the
trigger, Ampello removes it and inserts the content in its place.

```
:email      →   Hello,
                Thank you for reaching out.
                Best regards,
                Yohann

:report     →   See the attached files.
                📎 contract.pdf   📎 figures.xlsx   📎 chart.png
```

A snippet is a trigger, a body of text, and any files that go with it. Ampello
does not ask what kind of text it is, or what kind of file. One character or
fifty thousand are the same thing to the data model, as are a PNG, a PDF and a
`.docx`.

## Features

* **Works everywhere.** A system-wide keyboard hook, not a browser extension or
  an editor plugin.
* **File attachments.** A snippet can carry any files, delivered in the order
  you choose. Chat prompts and web forms receive them as uploads.
* **Fast.** Trigger matching costs about 31 ns per keystroke, measured on a
  library of ten thousand snippets.
* **Interruptible.** Press Escape while a long snippet is being inserted and it
  stops where it is.
* **Clipboard shortcut.** A second shortcut inserts the current clipboard as
  keystrokes, for applications that refuse or reformat a paste.
* **Local and private.** One SQLite file on your machine. No account, no sync,
  no telemetry, and no network client of any kind.
* **Shared libraries.** Accounts on the same machine can opt in to a shared
  snippet folder. Private by default.
* **Readable backups.** Exports are YAML you can diff and edit by hand.

## Installation

Download the latest installer from the
[releases page](https://github.com/ampello/ampello/releases) and run it.

| Architecture | File |
| --- | --- |
| x64 (most PCs) | `Ampello_<version>_x64-setup.exe` |
| ARM64 (Snapdragon, Surface Pro X) | `Ampello_<version>_arm64-setup.exe` |

The installer asks whether to install for the current user only (no
administrator rights required) or for all users on the machine. A portable
executable is published alongside the installers if you would rather not
install anything.

> **Note**
> Release binaries are not code-signed, so Windows SmartScreen will warn the
> first time you run one. Choose *More info* then *Run anyway*, verify the
> download against the published `SHA256SUMS`, or build from source.

### Requirements

Windows 10 (up to date) or Windows 11. The WebView2 runtime ships with both.

## Usage

Triggers fire when you finish them. Typing `:email` does nothing on its own;
`:email` followed by a space, a full stop, Tab or Enter expands it. Ampello
swallows that final keystroke, erases the trigger, inserts the content, then
sends the keystroke through, so `:email.` keeps its full stop.

Word boundaries are respected by default, so `something:sig` is left alone. You
can turn that off in Settings.

### Sharing a library between accounts

Every Windows account keeps its own library, private to that account. On a
shared machine you can point several accounts at one folder:

1. Open **Settings → Data → Location** and choose **Share**.
2. Pick a folder every account can reach. `C:\Users\Public\Ampello` is offered
   by default because Windows makes it writable by all accounts.
3. Restart Ampello, then repeat on each account that should join.

Nothing is shared until an account is pointed at the folder. Selecting **Use
personal** returns that account to its own library and leaves the shared one
untouched.

Snippets are not copied between libraries; use Export and Import to move them.
A running instance does not pick up another account's edits until it restarts,
which matters only when two accounts are signed in at once. Keep the folder on
a local disk, because SQLite's write-ahead log is not reliable over a network
share.

### Where data is stored

| | |
| --- | --- |
| Library | `%APPDATA%\com.yohann.ampello\ampello.db` |
| Attachments | `attachments\` beside the library |
| Log | shown in Settings → Data |

Attachments are stored by the SHA-256 of their contents, so the same file used
by ten snippets is stored once.

## Building from source

| Requirement | Notes |
| --- | --- |
| Node.js 20 or newer | `node --version` |
| Rust, stable toolchain | target `x86_64-pc-windows-msvc` |
| Visual Studio Build Tools | with "Desktop development with C++" |

```bash
git clone https://github.com/ampello/ampello.git
cd ampello
npm install
npm run tauri dev
```

To produce an installer:

```bash
npm run tauri build
```

The full check suite:

```bash
npm run typecheck
npm run build
cargo test  --manifest-path src-tauri/crates/ampello-core/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

> **Note**
> `tauri-build` does not watch the icon files. After changing artwork, run
> `cargo clean -p ampello` first, or the build will keep the previous icon
> without reporting anything.

## Project structure

```
src/                     React and TypeScript interface
  lib/                   types and the typed IPC boundary
  stores/                application state
  views/                 dashboard, snippets, settings, editor

src-tauri/
  src/                   window, tray, commands, Windows input
    library.rs           personal and shared library resolution
    input/win/           keyboard hook, injector, clipboard
  crates/ampello-core/   platform-independent core
    attachments.rs       content-addressed attachment store
    engine/              boundary rules, buffer, trigger matcher
    db/                  schema, migrations, settings
```

`ampello-core` has no Tauri dependency. It compiles and is tested on any
platform, which keeps a future macOS or Linux port from becoming a rewrite and
allows the storage and matching logic to be tested without a window.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how a keystroke becomes an
expansion, and [docs/TESTING.md](docs/TESTING.md) for the manual test plan.

## Known limitations

These follow from how Windows handles input, and none of them can be detected
in advance.

* **Elevated windows.** A normal process cannot observe or send keystrokes to a
  window running as administrator. Ampello must be elevated as well.
* **Secure input fields.** UAC prompts, the lock screen and some password
  managers are deliberately invisible to keyboard hooks.
* **Anti-cheat software and some games** read input below the hook layer, or
  reject synthetic input entirely.
* **Applications that ignore Ctrl+V**, such as older console hosts. Set
  Settings → Expansion → Insertion method to *Type*.
* **Attachments in plain text fields.** A field with no paste handler for files
  accepts nothing, without reporting an error.

Verified against Notepad, Notepad++, Chrome, Visual Studio Code, Windows
Terminal, Discord, Word, Slack and File Explorer.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md)
first, and open an issue before starting anything larger than a bug fix.

## Security

Ampello observes every keystroke while it runs. Please report security issues
privately rather than in a public issue. See [SECURITY.md](SECURITY.md) for the
reporting process and for a description of what the application can see.

## License

Ampello is released under the
[GNU General Public License v3.0 or later](LICENSE).

This program is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
PARTICULAR PURPOSE.
