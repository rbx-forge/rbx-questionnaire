# rbx-questionnaire

[![CI](https://github.com/rbx-forge/rbx-questionnaire/actions/workflows/ci.yml/badge.svg)](https://github.com/rbx-forge/rbx-questionnaire/actions/workflows/ci.yml)
[![License: MPL 2.0](https://img.shields.io/badge/license-MPL--2.0-blue.svg)](./LICENSE)

Declare, review and submit a Roblox experience's content questionnaire, the answers that decide its **age rating**, from a file you commit.

```sh
rbx-questionnaire pull   --env prod    # write the questions into rbxquestionnaire.toml
rbx-questionnaire check  --env prod    # every question answered? non-zero if not
rbx-questionnaire submit --env prod    # shows the resulting age rating, then asks
```

These are the Creator Hub's own endpoints, authenticated with a `.ROBLOSECURITY` cookie. Roblox does not document them, and they can change without notice.

## Nothing is defaulted

**An unanswered question stops a submission.** It is never filled in for you: "no" to everything is a true answer for many games and a false filing for others.

```toml
# rbxquestionnaire.toml

# Does your experience contain depictions of violence?
# options: Yes, No
[[question]]
id = "d5ef1b27-908d-7f3c-6842-7206aba90d76"
topic = "Violence"
answer = "no"
```

`id` is Roblox's question id, stable when the wording changes. `topic` is for people. `answer` matches an option exactly, then by option id, then by an unambiguous prefix: `yes` finds `Yes (Contains Violence)`, but `M` (both `Mild` and `Moderate`) is refused.

## Commands

| | |
| --- | --- |
| `pull` | Write or update the file from the live questionnaire. Only appends: never rewrites an answer or removes an entry, and keeps your comments |
| `show` | The questions, and your answer to each |
| `check` | Non-zero unless every question asked is answered. For CI |
| `submit` | Validate with Roblox, show the resulting age rating, ask, then file. `--dry-run` stops before filing, `--yes` skips the question |

To start from "no" rather than fill fourteen answers by hand:

```sh
$ rbx-questionnaire pull --env prod --fill no
Wrote rbxquestionnaire.toml with 26 question(s).
Answered 18 of them with 'no'. Read them before submitting: they are what your game declares.
Left blank, no matching option: AI Interaction, Blood, Crude Humor, Fear, Violence.
```

The answers land in the file, where a reviewer reads them. Existing answers are never overwritten.

Roblox keeps the last submission: run `check` in CI, and `submit` when the answers change.

## Follow-up questions

Some options reveal further questions, required like the rest. Answering "no" throughout reveals none, which is why a tool that only says "no" seems to work. Of 26 questions in a real questionnaire, 14 are reachable that way; a `yes` on violence adds two that are not yes/no:

```
$ rbx-questionnaire check --env prod
  'no' is not an option for: Violence
      options: Mild, Moderate
```

Follow-ups behind an option you did not choose are not required and not submitted, so flipping an answer keeps what you wrote under it.

## When Roblox changes the questionnaire

A new question shows as unanswered and blocks a submission. A removed question is reported (`no longer asked`) and blocks nothing.

## Target and credentials

`--universe-id` names the universe, or `--env prod` reads it from the project's `rbxplace.toml` (read only).

The cookie comes from `--cookie`, then `RBX_COOKIE`, then a signed-in local Roblox Studio. It needs edit access to the universe: a `403` is almost always that, or an expired cookie.

## Install

With [mise](https://mise.jdx.dev) or [Rokit](https://github.com/rojo-rbx/rokit), either of which pins the latest release in your project's config:

```bash
mise use --pin github:rbx-forge/rbx-questionnaire
rokit add rbx-forge/rbx-questionnaire   # or, with Rokit
```

Or take a binary from the [releases page](https://github.com/rbx-forge/rbx-questionnaire/releases), with a `SHA256SUMS` beside them, or build from source with `cargo install --git https://github.com/rbx-forge/rbx-questionnaire --locked`.

## License

[MPL-2.0](./LICENSE). A community tool, not affiliated with Roblox Corporation.
