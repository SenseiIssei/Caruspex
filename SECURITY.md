# Security policy

## Reporting a vulnerability

Please do not report security problems in a public issue, a pull request or a discussion.

Report them privately with GitHub's private vulnerability reporting: open the
[Security tab of the repository](https://github.com/SenseiIssei/Caruspex/security) and choose
**Report a vulnerability**, or go straight to
[the form for a new advisory](https://github.com/SenseiIssei/Caruspex/security/advisories/new).
Only the maintainers of Caruspex see the report. Reports in English or German are welcome.

Please include what is affected (version, system, which part of Caruspex), how to reproduce it, and what an
attacker could do with it. You will get an answer in the advisory. Once a fix is released, the advisory is
published, with credit to you if you want it.

## What counts

Caruspex runs on the user's computer or phone and talks to a car through an adapter. Examples of what we want to
hear about:

- the updater installing something that was not signed with the release key, or downloading from anywhere but the
  releases of this repository (`app/src-tauri/src/update.rs`)
- a file the user opens (a recording for replay, a custom PID CSV, a PID pack, a 3D model, a garage file) that
  makes Caruspex run code, read or write files it should not, or crash in a way that can be abused
- personal data leaving the computer without the user's action, or "Share recording" and the crash reports
  keeping a full VIN, an address or the user's home folder that they promise to remove
- any way for Caruspex to write to a car without the user's confirmation, or to send requests the safety rules in
  [CONTRIBUTING.md](CONTRIBUTING.md) rule out
- a problem in the CI that could let a pull request reach the release secrets

Weaknesses of cars, control units or adapters themselves are not problems of Caruspex; please report those to their
makers.

## Supported versions

Fixes go into the newest release, which the built-in updater offers to every installation. Older versions do not
get separate fixes.
