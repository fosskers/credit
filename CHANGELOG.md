# credit

## Unreleased

#### Added

- New `--start` and `--end` options for `repo` that will only consider
  contributions/comments between the given dates.
- Another contributor ranking has been added: the number of commits that appear
  in merged PRs.

#### Changed

- The commits-in-merged-PRs statistic changed some types, and thus would
  invalidate any output from `--json` given in `1.0.0`.

## 1.0.0 (2020-06-27)

#### Added

- New `repo` command that holds the old default behaviour. Use this to analyse
  projects.
- New `limit` command for reporting the remaining Github API query allowance for
  a given token.
- New `json` command for generating a full Markdown report for JSON results
  produced by a previous run of `credit repo --json`.

#### Changed

- `credit` now uses the GraphQL-based V4 Github API. This has drastically
  improved performance and uses far less of a user's API quota upon each run.

## 0.2.0 (2020-06-22)

#### Changed

- Allow multiple repositories to be checked at the same time. Their results are
  aggregated into a single report.

## 0.1.0 (2020-06-18)

The initial release.
