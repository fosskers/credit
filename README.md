# Credit

[![Build](https://github.com/fosskers/credit/workflows/Build/badge.svg)](https://github.com/fosskers/credit/actions)
[![](https://img.shields.io/crates/v/credit.svg)](https://crates.io/crates/credit)


`credit` - A tool for measuring Github repository contributions and the overall
health of a project.

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

> **💡 Note:** `credit` calls the Github API, which has a rate limit of 5,000
> requests per hour. If you use `credit` on too large of a project (5,000+
> combined Issues and Pull Requests), it will use up all your allotted requests
> and yield inaccurate results!
>
> Future developments will allow you to restrict your queries to certain time
> periods.

### Markdown Output

By default, `credit` outputs text to stdout that can be piped into a `.md` file
and displayed as you wish:

```
> credit --token=<token> fosskers/versions

# Project Report for versions

## Issues

This repo has had 7 issues, 6 of which are now closed (85.7%).

- 6 (85.7%) of these received a response.
- 6 (85.7%) have an official response from a repo Owner or organization Member.

Response Times (any):
- Median: 1 hour
- Average: 5 hours

Response Times (official):
- Median: 1 hour
- Average: 5 hours

## Pull Requests

This repo has had 19 Pull Requests, 8 of which are now merged (42.1%).
11 have been closed without merging (57.9%).

- 3 (15.8%) of these received a response.
- 3 (15.8%) have an official response from a repo Owner or organization Member.

Response Times (any):
- Median: 10 hours
- Average: 10 hours

Response Times (official):
- Median: 10 hours
- Average: 10 hours

Time-to-Merge:
- Median: 1 hour
- Average: 10 hours

## Contributors

Top 10 Commentors (Issues and PRs):
1. fosskers: 33
2. omgbebebe: 4
3. bergmark: 2
4. taktoa: 2
5. mightybyte: 2
6. hvr: 2
7. hasufell: 1
8. jaspervdj-luminal: 1

Top 10 Code Contributors (by merged PRs):
1. fosskers: 7
2. jaspervdj: 1
```

### JSON Output

You can also output the raw results as `--json`, which could then be piped to
tools like [`jq`](https://github.com/stedolan/jq) or manipulated as you wish:

```
> credit --token=<token> fosskers/versions --json

{"commentors":{"bergmark":2,"fosskers":33,"taktoa":2,"omgbebebe":4,"hvr":2,"jaspervdj-luminal":1,"mightybyte":2,"hasufell":1},"code_contributors":{"jaspervdj":1,"fosskers":7},"all_issues":7,"all_closed_issues":6,"issues_with_responses":6,"issues_with_official_responses":6,"issue_first_resp_time":{"median":{"secs":5962,"nanos":0},"mean":{"secs":21545,"nanos":0}},"issue_official_first_resp_time":{"median":{"secs":5962,"nanos":0},"mean":{"secs":21545,"nanos":0}},"all_prs":19,"prs_with_responses":3,"prs_with_official_responses":3,"pr_first_resp_time":{"median":{"secs":36335,"nanos":0},"mean":{"secs":39128,"nanos":0}},"pr_official_first_resp_time":{"median":{"secs":36335,"nanos":0},"mean":{"secs":39128,"nanos":0}},"prs_merged":8,"prs_closed_without_merging":11,"pr_merge_time":{"median":{"secs":6265,"nanos":0},"mean":{"secs":38530,"nanos":0}}}
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

### Median vs Mean

You may notice that sometimes the reported `Median` and `Average` results can be
wildly different. Given the presence of outliers in a data set, it can sometimes
be more accurate to consider the Median and not the Mean.

In the case of maintainer response times, consider a developer who usually
responds to all new Issues within 10 minutes. Then he goes on vacation, and
misses a few until his return 2 weeks later. His Average would be skewed in this
case, but the Median would remain accurate.

`credit` doesn't attempt to remove outliers, but might in the future.
