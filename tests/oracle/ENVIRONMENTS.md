# Oracle environments

Append-only record of the real-VBA environments whose results were applied to the corpus
(see `docs/tooling/differential-testing.md`). A program with `' SOURCE: oracle <env id> ...`
was confirmed by exactly that environment. VBA behavior depends on host version, build,
bitness and locale, so none of these may be omitted or edited after the fact.

| Env id | Host | Version | Build | VBA | Bitness | OS | Locale (LCID) | Captured | Upgraded | Not matching |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | ---: | ---: |
