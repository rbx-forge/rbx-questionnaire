# rbx-questionnaire

[![CI](https://github.com/rbx-dev-tools/rbx-questionnaire/actions/workflows/ci.yml/badge.svg)](https://github.com/rbx-dev-tools/rbx-questionnaire/actions/workflows/ci.yml)
[![License: MPL 2.0](https://img.shields.io/badge/license-MPL--2.0-blue.svg)](./LICENSE)

Declare, review and submit a Roblox experience's content questionnaire.

```sh
rbx-questionnaire pull   --env prod    # write the questions into a file
rbx-questionnaire check  --env prod    # every question answered? exits non-zero if not
rbx-questionnaire submit --env prod    # shows the resulting age rating, then asks
```

Roblox turns these answers into your experience's **age rating**. That makes the
questionnaire a statement about the game rather than a deployment setting, and
this tool is shaped around that one fact.

> **Not an Open Cloud API.** These endpoints are the ones the Creator Hub calls,
> authenticated with a `.ROBLOSECURITY` cookie, and Roblox does not document
> them. They can change without notice. That is why this is its own binary
> rather than a subcommand of an Open Cloud CLI: a spec-drift check cannot warn
> about an endpoint that is not in the spec, so breakage here costs one small
> release and nothing else.

## Nothing is ever defaulted

The rule the whole tool is built on: **an unanswered question stops a
submission.** It is never filled in for you, not with "no", not with anything.

A content declaration nobody wrote is not a declaration. Answering "no" to every
question is a real answer for many experiences and a false filing for others,
and a tool cannot tell which one it is looking at. So the answers live in a file
you commit, where a person can read them and a reviewer can disagree.

```toml
# rbxquestionnaire.toml

# Does your experience contain depictions of violence?
# options: Yes, No
[[question]]
id = "d5ef1b27-908d-7f3c-6842-7206aba90d76"
topic = "Violence"
answer = "no"
```

`id` is Roblox's own question id, which is what stays stable when a question is
reworded. `topic` is a label for people and is never matched on. `answer` is
matched against the option text.

The options are not always the words you would expect. The real violence
question offers `No` and `Yes (Contains Violence)`, and its follow-ups offer
`Mild / Moderate` and `Occasional / Repeated`. So an answer matches exactly
first, then by the option id, then by an unambiguous prefix: `yes` finds
`Yes (Contains Violence)`, while `M` fits both `Mild` and `Moderate` and is
refused rather than resolved to the first. Choosing between two options on your
behalf is choosing what your game declares.

## The commands

| | |
| --- | --- |
| `pull` | Write or update the file from the live questionnaire. **Only ever appends.** |
| `show` | The questions, and what your file answers to each. |
| `check` | Exits non-zero unless every question asked is answered. The CI contract. |
| `submit` | Validates, shows the resulting age rating, then asks before filing. |

`pull` never rewrites an answer and never removes an entry, because either would
change a declaration filed under your name without you saying so. Entries Roblox
no longer asks are reported by `check`, not deleted. It edits the document rather
than re-serializing it, so the note you left beside an answer survives.

### Starting from "no"

Most experiences answer no to most of it, and filling fourteen answers by hand to
say so is a poor first five minutes:

```sh
$ rbx-questionnaire pull --env prod --fill no
Wrote rbxquestionnaire.toml with 26 question(s).
Answered 18 of them with 'no'. Read them before submitting: they are what your game declares.
Left blank, no matching option: AI Interaction, Blood, Crude Humor, Fear, Violence.
```

This is a flag and not a default, and the difference is the whole point. The
answers land **in the file**, where they are committed, read in a diff, and
disagreed with by a reviewer. An answer nobody typed is not a declaration; a file
somebody generated and kept is. It never overwrites an answer already there, and
it leaves blank every question whose options are not the word you asked for.

Then, on every deploy after that, the file is the answer:

```sh
rbx-questionnaire check  --env prod          # in CI, exits non-zero if incomplete
rbx-questionnaire submit --env prod --yes    # no prompt, files what the file says
```

Roblox keeps the last submission, so this is not something to run on every
upload. `check` in CI is the part that belongs in a pipeline; `submit` belongs to
the day the answers change.

`submit` asks Roblox to validate first, because this tool's idea of "complete" is
not the one that counts, then prints the age rating the answers would produce
before anything is filed. `--dry-run` stops there. `--yes` skips the prompt, for
CI.

## Follow-up questions

Every option can carry questions that only exist once that option is chosen.
Declaring a `yes` reveals them, and they are as required as the rest:

```
$ rbx-questionnaire check --env prod
  unanswered: Violence
      Is the violence realistic?
      options: Yes, No
```

This is why a tool that only ever answers "no" appears to work: "no" reveals
nothing, so the follow-ups never have to be answered. The first honest `yes`
finds them. A follow-up behind an option you did not choose is not required, is
not submitted, and keeping an answer for it is fine, which is what lets you flip
a `yes` back and forth without losing what you wrote underneath.

The scale of it, measured against a real universe: the questionnaire holds **26
questions**, of which **14** are reachable if you answer no throughout. The other
twelve are follow-ups. Declaring one `yes` on violence pulls in two more, and
they are not yes/no questions:

```
$ rbx-questionnaire check --env prod
  'no' is not an option for: Violence
      options: Mild, Moderate
  'no' is not an option for: Violence
      options: Occasional, Repeated
```

## When Roblox changes the questionnaire

A question Roblox **adds** shows up as unanswered and blocks a submission. That
is the point of keying on ids rather than compiling a table of answers into the
binary: a hardcoded table goes stale silently, and the first thing you learn is
that you filed a declaration that did not cover a question you never saw.

A question Roblox **removes** is reported and blocks nothing. Your declaration is
still complete; the file merely has a line in it that nobody asks about now.

```
$ rbx-questionnaire check --env prod
  no longer asked: Gambling-Related Content (4991d5e0-...)
      harmless, but the questionnaire has changed since this was written
```

## Which universe

`--universe-id` names one directly. `--env prod` resolves it through
`rbxplace.toml`, the file [rbx-cli](https://github.com/rbx-forge/rbx-cli) owns,
so an environment name means the same universe to every tool in the project.
This tool only ever reads that file.

## Credentials

A `.ROBLOSECURITY` cookie, from `--cookie`, then `$RBX_COOKIE`, then a local
Roblox Studio install. The deliberate choice always beats the ambient one, and
the Studio fallback is what lets you use this without ever putting a cookie in a
shell history.

The cookie needs edit access to the universe. A 403 is almost always that, or an
expired cookie, and the error says so.

## Install

With [Rokit](https://github.com/rojo-rbx/rokit):

```toml
[tools]
rbx-questionnaire = "rbx-dev-tools/rbx-questionnaire@0.1.0"
```

then `rokit install`. Or take a binary from the
[releases page](https://github.com/rbx-dev-tools/rbx-questionnaire/releases),
which ships one zip per platform with a `SHA256SUMS` beside them, or build from
source with `cargo install --git`.

## Where this sits

| tool | does |
| --- | --- |
| **rbx-questionnaire** | **the content questionnaire and the age rating it produces** |
| [rbx-cli](https://github.com/rbx-forge/rbx-cli) | Open Cloud: metadata, shop, configs, place upload, live ops |
| [rbx-inject](https://github.com/rbx-dev-tools/rbx-inject) | writing asset ids into a `.rbxl` before upload |
| [rbx-observe](https://github.com/rbx-forge/rbx-observe) | public storefront data, other people's games |
| [rbx-switch](https://github.com/rbx-dev-tools/rbx-switch) | which Studio account the next command uses |

## License

[MPL-2.0](./LICENSE).
