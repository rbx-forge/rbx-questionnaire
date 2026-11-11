# Changelog

## [Unreleased]

### Security

- The API client's debug output no longer shows the session cookie or the CSRF token.

## [0.2.1] - 2026-11-10

### Security

- Release archives carry a build provenance attestation: `gh attestation verify <zip> --repo rbx-forge/rbx-questionnaire`. The build jobs run with read access only, and the publishing job alone may write.

### Fixed

- `submit` stops when Roblox marks the answers as not submittable. It printed that line and went on to the question, or straight to filing under `--yes`. `--dry-run` fails on it too, so a dry run that passes is one the real run would file.

## [0.2.0] - 2026-11-08

### Added

- `pull --fill <word>` answers every question that offers that option (`no`, typically), so a file can start from one word. Existing answers are never overwritten, and the questions without that option are listed blank.

### Changed

- **A signed-in Studio is used only after a yes.** The cookie of a local Roblox Studio used to be read whenever `--cookie` and `RBX_COOKIE` were absent. A session cookie is a whole account, so it is now asked for once on a terminal and refused where nobody can answer, like every other rbx-forge tool. `--auto-cookie` answers yes in advance, `--no-auto-cookie` refuses. In CI, set `RBX_COOKIE`.
- An empty `RBX_COOKIE=` now means "no cookie" and stops there, instead of falling through to Studio.
- The repository moved to the rbx-forge organisation.

## [0.1.0] - 2026-09-02

First release.

- `pull`, `show`, `check` and `submit` for an experience's content questionnaire, with the answers in `rbxquestionnaire.toml`.
- Nothing is defaulted: an unanswered question stops a submission.
- Follow-up questions are tracked, and questions Roblox adds or removes are reported.
