# credit

## 1.4.0 (2021-02-05)

#### Added

- A config file can now be defined at your `XDG_CONFIG_HOME`, which by default
  is `$HOME/.config/credit.toml`. At the moment the only field is `token`:

```toml
token = "abc123"  # Your Github Access Token.
```

With this, you no longer need to pass `--token` on the command line.

#### Changed

- The dependency `reqwest` has been removed in favour of raw `curl`. This
  reduces dependency count by about 100 crates, and the final stripped binary
  size is now 1.5mb, down from about 4.5mb.

## 1.3.0 (2020-08-24)

#### Added

- A `--version` flag to display the current version of `credit`.

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
