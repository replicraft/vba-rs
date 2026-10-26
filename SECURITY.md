# Security policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately through GitHub private vulnerability
reporting: open the repository's **Security** tab and choose **Report a vulnerability**.
Do not open a public issue or pull request for a security problem.

Include what you can of:

* the version or commit, and your platform;
* the input that triggers the problem (a project file, a source file or a command line),
  minimized if possible;
* what you expected and what happened (a crash, an escape from a grant or limit, a hang,
  unbounded memory use);
* whether you think it can be reached from untrusted input.

We will acknowledge the report, work with you on a fix and credit you if you wish.

## What counts

vba-rs is built to run VBA from untrusted files. These are in scope:

* code running without the embedding application trusting it, or with an effect the
  granted capabilities do not allow;
* an escape from a scoped grant (path traversal, links, alternative spellings);
* exceeding a resource limit the program cannot lift;
* a panic, hang or unbounded allocation caused by a malformed input file.

Read [docs/architecture/security-model.md](docs/architecture/security-model.md) first. It
lists what is guaranteed, with the tests, and what is explicitly not guaranteed (for
example, memory limits are accounting rather than a sandbox and the Wine helper executes
native code). A report about something listed there is still welcome as a hardening
suggestion but is not a vulnerability in the stated model.

## Supported versions

The project is at version 0.1.0 and has no stable releases. Fixes go to the main branch.
