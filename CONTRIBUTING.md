# Contributing to Ampello

Thanks for taking an interest. Bug reports, fixes and focused features are all
welcome.

## Before you start

For anything larger than a bug fix, **open an issue first**. Ampello has a
deliberately narrow design — described below — and a pull request that works
against it is a waste of your time and ours.

## Getting set up

You need Node.js 20+, a stable Rust toolchain for `x86_64-pc-windows-msvc`, and
Visual Studio Build Tools with "Desktop development with C++".

```bash
npm install
npm run tauri dev
```

Before opening a pull request, all four of these must pass:

```bash
npm run typecheck
npm run build
cargo test --manifest-path src-tauri/crates/ampello-core/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

Anything touching the keyboard hook, the injector or the clipboard also needs a
manual pass — see [docs/TESTING.md](docs/TESTING.md). Those paths cannot be
covered by automated tests, because they depend on how other applications
behave.

## The design, and what it rules out

Ampello stores text and replays text. Please do not propose:

- **Content classification.** No content-type field, no language picker, no
  editor modes. A one-character snippet and a fifty-thousand-character one are
  the same row in the same table; a PNG and a `.docx` take the same path.
- **Executing snippets.** No shell syntax, no scripting, no evaluation. A
  snippet is data, never code. This is a security boundary, not a taste.
- **Altering content.** No trimming, no whitespace normalisation. Content round
  trips byte for byte, including tabs, trailing spaces, CRLF and emoji.
- **Network features.** No account, no sync, no telemetry. Ampello has no
  network client and should not gain one.

## House style

**Rust and TypeScript.** `cargo fmt` and the surrounding file's conventions.
Match what is already there rather than introducing a new pattern.

**Comments are rare on purpose.** Write one only where the code cannot say it
itself — a Windows quirk, a constraint that a reasonable change would break, a
security boundary. Do not write comments that restate the line below them.

**Platform code stays out of the core.** `ampello-core` has no Tauri and no
Win32 dependency, and must keep compiling and testing on any platform. Anything
touching the hook, injection, the clipboard or the tray belongs in
`src-tauri/src`.

**New files carry an SPDX header:**

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
```

## Commits and pull requests

Write commit subjects in the imperative — "Fix trigger matching across word
boundaries", not "Fixed…". Explain *why* in the body when it is not obvious.

Keep a pull request to one change. Say what you tested and on what — Windows
version, and which target applications if you touched the input path.

## Reporting bugs

Include your Windows version, your Ampello version, the application you were
typing into, and what you expected against what happened. If expansion stopped
working entirely, Settings → Expansion shows the keystroke counter — a stalled
counter means Windows removed the hook, which is a different bug from a trigger
that does not match.

Please do not paste snippet contents you would not want public.

## Security

Do not report security issues in a public issue. See [SECURITY.md](SECURITY.md).

## License

Ampello is GPL-3.0-or-later. By contributing you agree that your contribution
is licensed under the same terms.
