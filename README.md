<div align="center">

<img src="src-tauri/icons/128x128.png" width="88" height="88" alt="Ampello">

# Ampello

**A privacy-first, cross-platform snippet expansion tool**

[![License](https://img.shields.io/badge/license-GPLv3-6135e8.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/ampello/ampello?color=6135e8)](https://github.com/ampello/ampello/releases)
[![CI](https://github.com/ampello/ampello/actions/workflows/ci.yml/badge.svg)](https://github.com/ampello/ampello/actions/workflows/ci.yml)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-6135e8.svg)](#requirements)

[Installation](#installation) · [Features](#features) · [Documentation](docs/ARCHITECTURE.md) · [Contributing](CONTRIBUTING.md)

</div>

---

## Overview

Ampello is a text expander for Windows. It monitors keyboard input
system-wide, detects user-defined abbreviations, and substitutes them with
their associated content. Expansion is available in any application that
accepts keyboard input, including browsers, editors, terminals, email clients
and chat applications.

A snippet consists of a trigger, a body of text, and an optional ordered set of
file attachments. Content is stored verbatim and reproduced without
modification: no trimming, no whitespace normalisation, and no length limit
beyond available storage.

```
Trigger      Expansion
:email       Hello,
             Thank you for reaching out.

             Best regards,
             Yohann

:report      See the attached files.
             contract.pdf, figures.xlsx, chart.png
```

## Features

* **System-wide expansion.** Implemented as a low-level keyboard hook rather
  than a browser extension or an editor plugin, so a single library serves
  every application.
* **File attachments.** Snippets may carry arbitrary files, delivered in a
  defined order. Applications that accept pasted files, such as web upload
  fields and chat composers, receive them as attachments.
* **Predictable performance.** Trigger matching costs approximately 31 ns per
  keystroke against a library of ten thousand snippets, measured on a release
  build.
* **Cancellable insertion.** Pressing Escape during a long insertion halts it
  at the current position.
* **Clipboard shortcut.** A separate global shortcut inserts the current
  clipboard contents as synthetic keystrokes, for applications that reject or
  reformat a standard paste.
* **Local storage only.** Data is held in a single SQLite database on the local
  machine. Ampello has no account system, no synchronisation service and no
  network client.
* **Optional shared libraries.** Accounts on the same machine may opt in to a
  common snippet directory. Libraries are private to each account by default.
* **Portable backups.** Exports are plain YAML, suitable for version control
  and manual editing.

## Installation

Download the installer for your architecture from the
[releases page](https://github.com/ampello/ampello/releases).

| Architecture | File |
| --- | --- |
| x64 | `Ampello_<version>_x64-setup.exe` |
| ARM64 | `Ampello_<version>_arm64-setup.exe` |

The installer offers two installation scopes. A per-user installation requires
no elevated privileges. A machine-wide installation requires administrator
rights and makes the application available to every account. Portable
executables are published alongside the installers.

> **Note**
> Release binaries are unsigned, so Windows SmartScreen displays a warning on
> first execution. Select *More info* followed by *Run anyway*, verify the
> download against the published `SHA256SUMS`, or build from source.

### Requirements

Windows 10 (current servicing baseline) or Windows 11. The WebView2 runtime is
included with both.

## Usage

A trigger is evaluated once it has been terminated. Typing `:email` has no
effect on its own; expansion occurs when the trigger is followed by a space, a
full stop, Tab or Enter. Ampello suppresses that terminating keystroke, removes
the trigger, inserts the replacement content, then forwards the original
keystroke, so `:email.` retains its trailing punctuation.

Triggers are matched at word boundaries by default, so `something:sig` is not
expanded. This behaviour can be disabled in Settings.

### Shared libraries

Each Windows account maintains an independent library. On a shared machine,
several accounts may be configured to use a common directory:

1. Open **Settings → Data → Location** and select **Share**.
2. Choose a directory accessible to every participating account.
   `C:\Users\Public\Ampello` is proposed by default, as Windows grants write
   access to all accounts there without further configuration.
3. Restart Ampello and repeat the procedure for each account.

No data is shared until an account is explicitly configured to use the shared
directory. Selecting **Use personal** returns that account to its own library
and leaves the shared one unmodified.

Three constraints apply. Snippets are not copied between libraries; use Export
and Import to transfer them. A running instance does not observe changes made
by another account until it restarts, which is relevant only when several
accounts are signed in concurrently. The directory must reside on a local
volume, as SQLite's write-ahead log is not reliable over network shares.

### Data locations

| Item | Path |
| --- | --- |
| Library | `%APPDATA%\com.yohann.ampello\ampello.db` |
| Attachments | `attachments\` adjacent to the library |
| Log | reported in Settings → Data |

Attachments are content-addressed by SHA-256, so a file referenced by several
snippets is stored once.

## Building from source

| Requirement | Notes |
| --- | --- |
| Node.js 20 or later | `node --version` |
| Rust, stable toolchain | target `x86_64-pc-windows-msvc` |
| Visual Studio Build Tools | including "Desktop development with C++" |

```bash
git clone https://github.com/ampello/ampello.git
cd ampello
npm install
npm run tauri dev
```

To produce a distributable installer:

```bash
npm run tauri build
```

The complete verification suite:

```bash
npm run typecheck
npm run build
cargo test  --manifest-path src-tauri/crates/ampello-core/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

> **Note**
> Close Ampello before building. A running instance retains a handle on
> `src-tauri/target/release/ampello.exe`, and the build terminates with
> `Access is denied. (os error 5)`. Exit from the tray icon, or run
> `Stop-Process -Name ampello`.

> **Note**
> `tauri-build` does not declare a dependency on the icon files. After
> replacing artwork, run `cargo clean -p ampello`; otherwise the build retains
> the previous icon without reporting an error.

## Project structure

```
src/                     React and TypeScript interface
  lib/                   shared types and the typed IPC boundary
  stores/                application state
  views/                 dashboard, snippets, settings, editor

src-tauri/
  src/                   window management, tray, commands, Windows input
    library.rs           personal and shared library resolution
    input/win/           keyboard hook, injector, clipboard
  crates/ampello-core/   platform-independent core
    attachments.rs       content-addressed attachment store
    engine/              boundary rules, input buffer, trigger matcher
    db/                  schema, migrations, settings
```

`ampello-core` carries no dependency on Tauri or the Windows API. It compiles
and is tested on any platform, which keeps a future macOS or Linux port from
requiring a rewrite and allows the storage and matching logic to be tested
without a graphical environment.

Refer to [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the expansion
pipeline and to [docs/TESTING.md](docs/TESTING.md) for the manual test plan.

## Known limitations

The following are consequences of the Windows input model and cannot be
detected programmatically in advance.

* **Elevated windows.** A process running at standard integrity cannot observe
  or inject keystrokes for a window running as administrator. Ampello must be
  elevated to operate in such windows.
* **Protected input contexts.** UAC prompts, the lock screen and certain
  password managers are deliberately inaccessible to keyboard hooks.
* **Anti-cheat software and some games** intercept input below the hook layer
  or reject synthetic input entirely.
* **Applications that do not honour Ctrl+V**, such as older console hosts. Set
  Settings → Expansion → Insertion method to *Type*.
* **Attachments in plain text fields.** A field without a file paste handler
  accepts nothing and reports no error.

Verified against Notepad, Notepad++, Chrome, Visual Studio Code, Windows
Terminal, Discord, Word, Slack and File Explorer.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md)
before submitting changes, and open an issue to discuss anything beyond a bug
fix.

## Security

Ampello observes all keyboard input while running. Report security issues
privately rather than through the public issue tracker. See
[SECURITY.md](SECURITY.md) for the disclosure process and for a description of
what the application can access.

## License

Ampello is distributed under the
[GNU General Public License v3.0 or later](LICENSE).

This program is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
PARTICULAR PURPOSE.
