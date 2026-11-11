# Changelog

## [Unreleased]

## [0.3.0] - 2026-11-11

### Changed

- **`submit` names the experience and the account in its question.** It asked to file "for universe" and a number, and nothing said which account the cookie belonged to. The question now gives the experience's name and the signed-in account's name, and says so when either could not be read, then asks all the same.

### Fixed

- **`check` fails on a questionnaire with no question in it.** An empty answer from Roblox, or one in a shape this tool does not read, passed as "0 question(s) asked, 0 answered". It now exits non-zero and says no question could be read.
- **The age rating `submit` prints is the rating, never a descriptor.** It is read where Roblox puts it, under `ageRecommendationSummary`, and the search used for any other shape no longer looks among the descriptors, which have a display name of their own ("Violence").
- **`pull --fill no` fills only the options that say "No".** The fill word matched by prefix, so it also answered "None of the above" or "Not applicable". It now matches an option's whole text, ignoring case. A typed answer keeps its prefix rule.

## [0.2.3] - 2026-11-11

### Security

- **A redirect from Roblox is refused, never followed.** These endpoints answer where they are asked, so a 3xx now fails the command with its status and the host it pointed to, and nothing more: not its body, not the rest of the address, which can be a signed link. Followed, it sent the CSRF token to that host, and turned a `submit` into a `GET` there whose answer could read as a filed declaration.

### Changed

- Built on rbx-core 0.2.0. The cookie chain this tool takes from it is unchanged: `--cookie` or `RBX_COOKIE` first, an empty value is no cookie, and a signed-in Studio is used only after a yes, `--auto-cookie` or `--no-auto-cookie` answering in advance.

## [0.2.2] - 2026-11-11

### Security

- `rustls` 0.23.46, `rustls-webpki` 0.103.16 and `tokio` 1.54.1, for RUSTSEC-2026-0285, -0346, -0348, -0349, -0382 and -0386. The one to read is -0382: certificate verification could accept distrusted intermediates, on the connection that carries the session cookie.
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
