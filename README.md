# Credit

[![Build](https://github.com/fosskers/credit/workflows/Build/badge.svg)](https://github.com/fosskers/credit/actions)
[![](https://img.shields.io/crates/v/credit.svg)](https://crates.io/crates/credit)
![AUR version](https://img.shields.io/aur/version/credit-bin)

`credit` - A fast tool for measuring Github repository contributions and the
overall health of a project.

Use `credit` to find out:

- Who has the most Pull Requests merged to a project.
- Who engages in the most discussion in Issues and PRs.
- How long it takes maintainers to respond to and solve Issues.
- How long it takes to get PRs merged.
- If a library would be a safe long-term (i.e. maintained) dependency.

## Installation

### Arch Linux

With an AUR-compatible package manager like
[`aura`](https://github.com/fosskers/aura):

```
sudo aura -A credit-bin
```

### Cargo

```
cargo install credit
```

## Usage

To use `credit`, you'll need a Github [Personal Access
Token](https://github.com/settings/tokens) with `public_repo` permissions. [See
here](https://github.com/fosskers/active#oauth) for an additional example.

> **💡 Note:** `credit` calls the GraphQL-based Github v4 API, which has a much
> higher rate limit than the REST-based v3 API. This allows `credit` to run
> quickly and work on projects with a long development history.
>
> You can use `credit limit` to check your current API query allowance.

### Markdown Output

By default, `credit` outputs text to stdout that can be piped into a `.md` file
and displayed as you wish:

```
> credit repo --token=<token> rust-lang/rustfmt

# Project Report for rustfmt

## Issues

2462 issues found, 2189 of which are now closed (88.9%).

- 1899 (77.1%) of these received a response.
- 1553 (63.1%) have an official response from a repo Owner or organization Member.

Response Times (any):
- Median: 10 hours
- Average: 34 days

Response Times (official):
- Median: 13 hours
- Average: 39 days

## Pull Requests

1821 Pull Requests found, 1650 of which are now merged (90.6%).
168 have been closed without merging (9.2%).

- 1505 (82.6%) of these received a response.
- 1379 (75.7%) have an official response from a repo Owner or organization Member.

Response Times (any):
- Median: 8 hours
- Average: 2 days

Response Times (official):
- Median: 12 hours
- Average: 2 days

Time-to-Merge:
- Median: 17 hours
- Average: 3 days

## Contributors

Top 10 Commentors (Issues and PRs):
1. nrc: 2772
2. topecongiro: 1526
3. marcusklaas: 718
4. calebcartwright: 461
5. scampi: 331
6. kamalmarhubi: 120
7. rchaser53: 103
8. cassiersg: 100
9. gnzlbg: 79
10. otavio: 63

Top 10 Code Contributors (by merged PRs):
1. topecongiro: 513
2. marcusklaas: 125
3. calebcartwright: 74
4. nrc: 72
5. scampi: 64
6. rchaser53: 57
7. davidalber: 34
8. kamalmarhubi: 31
9. ayazhafiz: 28
10. sinkuu: 24
```

### JSON Output

You can also output the raw results as `--json`, which could then be piped to
tools like [`jq`](https://github.com/stedolan/jq) or manipulated as you wish:

```
> credit repo --token=<token> rust-lang/rustfmt --json
```

### Large Projects
By default, `credit` queries for Issues and Pull Requests at the same time,
which is fast and works well for most projects. For *very* large projects,
however, this can make the Github API unhappy.

If you notice `credit` failing on projects ones with many thousands of Issues
and Pull Requests, consider the `--serial` flag. This will pull Issues first,
and then Pull Requests. `--serial` allows `credit` to even work on the [Rust
compiler](https://github.com/rust-lang/rust) itself!

```
> credit repo --token=<token> rust-lang/rust --serial
```

## Caveats

### Accuracy

The numbers given by `credit` are not perfect measures of developer productivity
nor maintainer responsiveness. Please use its results in good faith.

**Response Times:** Particularly in the Open Source world, volunteer developers
are under no obligation to respond in a time frame that is most convenient for
us the users.

**Merged PRs:** Without human eyes to judge a code contribution, its importance
can be difficult to measure. Some PRs are long, but do little. Some PRs are only
a single commit, but save the company. `credit` takes the stance that, over
time, with a large enough sample size, general trends of "who's doing the work"
will emerge. **Expect weird results** for one-man projects or projects that
otherwise have a long history of pushing directly to `master` without using PRs.

> Why not use commit counts instead of PRs?

Per-user commit counts are already available on Github.

### Median vs Mean

You may notice that sometimes the reported `Median` and `Average` results can be
wildly different. Given the presence of outliers in a data set, it can sometimes
be more accurate to consider the Median and not the Mean.

In the case of maintainer response times, consider a developer who usually
responds to all new Issues within 10 minutes. Then he goes on vacation, and
misses a few until his return 2 weeks later. His Average would be skewed in this
case, but the Median would remain accurate.

`credit` doesn't attempt to remove outliers, but might in the future.
