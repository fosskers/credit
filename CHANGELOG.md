# credit

## Unreleased

#### Changed

- New `repo` command that holds the old default behaviour. Use this to analyse
  projects.
- New `limit` command for reporting the remaining Github API query allowance for
  a given token.
- New `json` command for generating a full Markdown report for JSON results
  produced by a previous run of `credit repo --json`.
- `credit` now uses the GraphQL-based V4 Github API. This has drastically
  improved performance and uses far less of a user's API quota upon each run.

## 0.2.0 (2020-06-22)

#### Changed

- Allow multiple repositories to be checked at the same time. Their results are
  aggregated into a single report.

## 0.1.0 (2020-06-18)

The initial release.
