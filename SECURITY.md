# Security policy

## Reporting a vulnerability

**Please do not open a public issue for a security problem.**

Report it privately through
[GitHub Security Advisories](https://github.com/ampello/ampello/security/advisories/new),
which lets us discuss and fix the issue before it is public.

Please include what the issue is, how to reproduce it, which version you were
running, and what an attacker could do with it. A proof of concept helps but is
not required.

You should get an acknowledgement within a week. Ampello is a small
volunteer-maintained project, so please be patient with the fix itself — we
will keep you updated and credit you in the advisory unless you would rather we
did not.

## Supported versions

Only the latest release receives security fixes. There are no long-term support
branches.

## What Ampello can see, and what it does with it

Ampello installs a system-wide low-level keyboard hook. That is the entire
mechanism behind text expansion, and it means the application observes every
keystroke on the machine while it is running. This section exists so you can
judge the risk honestly.

**What it keeps.** A rolling in-memory buffer, no longer than the longest
trigger in your library. It is discarded on every mouse click, arrow key,
window change and shortcut. It is never written to disk.

**What it never does.** Nothing you type is recorded, logged or transmitted.
The log records what Ampello did — "expansion of snippet `<id>` failed" — never
what you typed, and never a snippet's contents. There is no account, no sync,
no telemetry and no network client of any kind.

**What is on disk.** Your snippet library is a plain, unencrypted SQLite file
in your Windows user profile, with attachments beside it. It is protected by
your account's file permissions and nothing more. Anyone with administrator
rights on the machine, or with your unlocked session, can read it. **Do not
store passwords, recovery codes or secrets in snippets.**

**Clipboard.** Pasting a snippet means putting it on the clipboard first.
Ampello copies out the existing clipboard contents, replaces them, and puts
them back afterwards. During that window — typically under a second — the
snippet is readable by any other application on the machine, as is anything on
the clipboard at any other time.

**Shared libraries.** A shared library is an ordinary folder. Every account
that can read the folder can read every snippet and attachment in it, and every
account that can write to it can change them. Windows file permissions are the
only boundary. Do not put anything in a shared library you would not show to
everyone who uses the machine.

**Elevated windows.** Ampello cannot see or send keystrokes to a window running
as administrator unless Ampello is elevated too. Running it elevated widens
what a compromise of Ampello would reach; only do so if you need expansion
inside elevated applications.

## Scope

In scope: anything that lets a snippet, a backup file, an attachment or an
imported archive execute code, escape the attachment store, read files outside
the library, or escalate privileges. Also anything that causes Ampello to
record or transmit typed input.

Out of scope: the fact that a keyboard hook sees keystrokes, that the library
is unencrypted, or that another administrator on the machine can read your
data. Those are documented above and are properties of the design, not defects.

## Distribution

Release binaries are built by GitHub Actions from a tagged commit. They are
**not code-signed**, so Windows SmartScreen will warn on first run. Verify the
SHA-256 checksums published with each release, or build from source.
