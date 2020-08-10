# credit

## 1.2.1 (2020-08-10)

#### Changed

- Swapped `isahc` for `reqwest`, which allows `credit` to be compiled with the
  MUSL target and be fully statically linked.

## 1.2.0 (2020-07-22)

#### Added

- New `users` command to produce per-country Developer Rankings.

```
> credit users --token=<token> --location=Switzerland

# Top 100 Open Source Contributors in Switzerland

There are currently 18518 Github users in Switzerland.

  1. oleg-nenashev (7331 contributions)
  2. cclauss (6378 contributions)
  3. dpryan79 (5604 contributions)
  4. peterpeterparker (4869 contributions)
  5. ReneNyffenegger (4722 contributions)
... and so on.
```

## 1.1.1 (2020-07-18)

#### Changed

- Better release profile which produces smaller binaries.

## 1.1.0 (2020-06-28)

#### Added

- New `--start` and `--end` options for `repo` that will only consider
  contributions/comments between the given dates.
- New `--commits` option for `repo`. This adds another contributor ranking: the
  number of commits that appear in merged PRs. Keep in mind that using this
  requires more data from Github, and so takes longer to complete.

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
