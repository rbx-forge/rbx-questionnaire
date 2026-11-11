# Security policy

`rbx-questionnaire` handles a Roblox session cookie, a whole account session:
the Creator Hub endpoints it calls take nothing else. And `submit` files a
universe's answers with Roblox. That makes the threat model worth stating.

## Reporting a vulnerability

Report it privately, through GitHub: on this repository, open the **Security**
tab and choose **Report a vulnerability**, or go straight to
<https://github.com/rbx-forge/rbx-questionnaire/security/advisories/new>.

Do not open a public issue for a vulnerability. Do not paste a real cookie
into a report: a redacted reproduction is always enough.

What helps: the `rbx-questionnaire --version`, the command you ran, what you
expected, what happened, and why it is a security problem rather than a bug.
A proof of concept is welcome but never required.

**What to expect.** This is a solo-maintained project. An acknowledgement
within a week is the realistic commitment; a fix lands as fast as the severity
warrants. You will be credited in the advisory and the changelog unless you
ask otherwise. There is no bounty program.

## Supported versions

Only the latest release. At 0.x, fixes go out as a new release rather than
being backported.

## What counts

In scope:

- The cookie written to disk, logged, printed, or sent anywhere other than
  the Roblox hosts the command is talking to.
- The Studio cookie read without asking, or on a run that has nobody to ask.
- Answers filed by `--dry-run`, or filed without the confirmation `submit`
  asks for.
- Dependency vulnerabilities with a plausible path to any of the above.

Out of scope:

- The cookie doing what the account it belongs to may do.
- Changes on Roblox's side to the undocumented endpoints this tool calls.
  Report Roblox-side behavior to Roblox.
- Anything requiring an attacker who already has your credentials, or write
  access to your machine or repository.

## Verifying a download

Each release archive carries a build provenance attestation, a signed
statement of which workflow run and commit produced it:

```sh
gh attestation verify rbx-questionnaire-v0.0.0-x86_64-unknown-linux-gnu.zip --repo rbx-forge/rbx-questionnaire
```
