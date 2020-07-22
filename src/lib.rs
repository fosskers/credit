//! A library for measuring Github repository contributions.

mod contribs;
mod github;
mod limit;
mod repo;

// Re-export.
pub use limit::rate_limit;

use anyhow::Context;
use chrono::{DateTime, Utc};
use counter::Counter;
use indicatif::ProgressBar;
use isahc::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// A nicer collated form of the data pulled from Github regarding User
/// Contributions.
#[derive(Serialize)]
pub struct UserContribs {
    pub total_users: u32,
    pub contributions: Vec<User>,
}

/// A user and their contributions.
#[derive(Serialize)]
pub struct User {
    pub login: String,
    pub name: Option<String>,
    pub public_contributions: u32,
}

impl From<contribs::UserContribs> for User {
    fn from(uc: contribs::UserContribs) -> Self {
        let public_contributions = uc.contribs();
        User {
            login: uc.login,
            name: uc.name,
            public_contributions,
        }
    }
}

/// Any type that contains a `Thread`.
trait Threaded {
    fn the_thread(&self) -> &Thread;
}

/// A Github Issue.
#[derive(Debug)]
pub struct Issue(Thread);

impl Threaded for Issue {
    fn the_thread(&self) -> &Thread {
        &self.0
    }
}

/// A Github Pull Request.
#[derive(Debug)]
pub struct PR {
    pub thread: Thread,
    pub commits: usize,
    pub merged: Option<DateTime<Utc>>,
}

impl PR {
    /// Was this Pull Request merged?
    pub fn is_merged(&self) -> bool {
        self.merged.is_some()
    }

    /// Was this Pull Request closed without merging?
    pub fn is_closed_not_merged(&self) -> bool {
        self.thread.closed.is_some() && !self.is_merged()
    }
}

impl Threaded for PR {
    fn the_thread(&self) -> &Thread {
        &self.thread
    }
}

/// A thread of conversation on Github.
///
/// This could either be associated with an Issue or a PR.
#[derive(Debug)]
pub struct Thread {
    /// Who opened the thread?
    pub author: String,
    /// When was the thread opened?
    pub posted: DateTime<Utc>,
    /// If it's already closed, when was it?
    pub closed: Option<DateTime<Utc>>,
    /// Who responded first?
    pub first_responder: Option<String>,
    /// When, if ever, was the first response?
    pub first_response: Option<DateTime<Utc>>,
    /// When, if ever, was there an "official" response?
    pub first_official_response: Option<DateTime<Utc>>,
    /// Comment counts of everyone who participated.
    pub comments: HashMap<String, usize>,
}

/// A collection of Issue and Pull Request [`Thread`](struct.Thread.html)s.
#[derive(Debug)]
pub struct Postings {
    pub issues: Vec<Issue>,
    pub prs: Vec<PR>,
}

impl Postings {
    /// Combine the results of two repository lookups.
    pub fn combine(self, other: Postings) -> Postings {
        let mut issues = self.issues;
        let mut prs = self.prs;

        issues.extend(other.issues);
        prs.extend(other.prs);

        Postings { issues, prs }
    }

    /// Consumes the `Postings` to form all the statistics.
    pub fn statistics(self) -> Statistics {
        let all_issues = self.issues.len();

        let all_closed_issues = self.issues.iter().filter(|i| i.0.closed.is_some()).count();

        let issues_with_responses = self
            .issues
            .iter()
            .filter_map(|i| i.0.first_response)
            .count();

        let issues_with_official_responses = self
            .issues
            .iter()
            .filter_map(|i| i.0.first_official_response)
            .count();

        let issue_first_resp_time = self.resp_times(|| self.issues.iter(), |i| i.0.first_response);

        let issue_official_first_resp_time =
            self.resp_times(|| self.issues.iter(), |i| i.0.first_official_response);

        let all_prs = self.prs.len();

        let prs_with_responses = self
            .prs
            .iter()
            .filter_map(|p| p.thread.first_response)
            .count();

        let prs_with_official_responses = self
            .prs
            .iter()
            .filter_map(|p| p.thread.first_official_response)
            .count();

        let pr_first_resp_time = self.resp_times(|| self.prs.iter(), |p| p.thread.first_response);

        let pr_official_first_resp_time =
            self.resp_times(|| self.prs.iter(), |p| p.thread.first_official_response);

        let prs_merged = self.prs.iter().filter(|p| p.is_merged()).count();

        let prs_closed_without_merging =
            self.prs.iter().filter(|p| p.is_closed_not_merged()).count();

        let pr_merge_time = self.resp_times(|| self.prs.iter(), |p| p.merged);

        let mut contributor_commits = HashMap::new();
        self.prs
            .iter()
            .filter(|p| p.merged.is_some())
            .for_each(|p| {
                let counter = contributor_commits
                    .entry(p.thread.author.clone())
                    .or_insert(0);
                *counter += p.commits;
            });

        let code_contributors = self
            .prs
            .iter()
            .filter(|p| p.merged.is_some())
            .map(|p| p.thread.author.clone()) // Naughty clone.
            .collect::<Counter<_>>()
            .into_map();

        let commentors = hashmap_combine(
            self.issues
                .into_iter()
                .map(|i| i.0.comments)
                .fold(HashMap::new(), hashmap_combine),
            self.prs
                .into_iter()
                .map(|p| p.thread.comments)
                .fold(HashMap::new(), hashmap_combine),
        );

        Statistics {
            commentors,
            code_contributors,
            contributor_commits,
            all_issues,
            all_closed_issues,
            issues_with_responses,
            issues_with_official_responses,
            issue_first_resp_time,
            issue_official_first_resp_time,
            all_prs,
            prs_merged,
            prs_closed_without_merging,
            prs_with_responses,
            prs_with_official_responses,
            pr_first_resp_time,
            pr_official_first_resp_time,
            pr_merge_time,
        }
    }

    /// Gather the mean/median response times in a generic way.
    fn resp_times<'a, F, G, T, A>(&self, f: F, g: G) -> Option<ResponseTimes>
    where
        F: FnOnce() -> T,
        G: Fn(&A) -> Option<DateTime<Utc>>,
        T: Iterator<Item = &'a A>,
        A: 'a + Threaded,
    {
        let resp_times: Vec<chrono::Duration> = f()
            .filter_map(|t| g(t).map(|r| r - t.the_thread().posted))
            .sorted()
            .collect();

        if resp_times.is_empty() {
            None
        } else {
            // `to_std` will error if the `Duration` is less than 0. That shouldn't
            // happen, since comments should always occur after the initial posting
            // time of the thread.
            let median = resp_times
                .get(resp_times.len() / 2)
                .and_then(|t| t.to_std().ok())?;
            let mean: i64 = resp_times.iter().map(|d| d.num_seconds()).sum();
            let mean = Duration::from_secs(mean as u64 / resp_times.len() as u64);

            Some(ResponseTimes { median, mean })
        }
    }
}

/// Statistics involving [`Thread`](struct.Thread.html) response times.
#[derive(Debug, Deserialize, Serialize)]
pub struct ResponseTimes {
    pub median: Duration,
    pub mean: Duration,
}

impl ResponseTimes {
    /// A human-friendly report of the median reponse time.
    pub fn median_time(&self) -> String {
        ResponseTimes::period(&self.median)
    }

    /// A human-friendly report of the average reponse time.
    pub fn average_time(&self) -> String {
        ResponseTimes::period(&self.mean)
    }

    fn period(duration: &Duration) -> String {
        let hours = duration.as_secs() / 3600;
        let (num, period) = if hours > 48 {
            (hours / 24, "days")
        } else if hours > 1 {
            (hours, "hours")
        } else if hours == 1 {
            (1, "hour")
        } else {
            (duration.as_secs() / 60, "minutes")
        };
        format!("{} {}", num, period)
    }
}

/// Various compiled statistics regarding contributions to a Github repository.
///
/// For the relevant fields below, an "official" response is any made by a
/// repository Owner, an organization Member, or an invited Collaborator.
#[derive(Debug, Deserialize, Serialize)]
pub struct Statistics {
    /// All issue/PR commentors.
    pub commentors: HashMap<String, usize>,
    /// All users who had PRs merged.
    pub code_contributors: HashMap<String, usize>,
    /// The commits-in-merged-PRs count for each user.
    #[serde(default)]
    pub contributor_commits: HashMap<String, usize>,
    /// The count of all issues, opened or closed.
    pub all_issues: usize,
    /// How many of the issues have been closed?
    pub all_closed_issues: usize,
    /// All issues that have been responded to in some way.
    pub issues_with_responses: usize,
    /// All issues that have been responded to "officially".
    pub issues_with_official_responses: usize,
    /// How long does it take for someone to respond to an issue?
    pub issue_first_resp_time: Option<ResponseTimes>,
    /// How long does it take for an "official" response?
    pub issue_official_first_resp_time: Option<ResponseTimes>,
    /// The count of all PRs, opened or closed.
    pub all_prs: usize,
    /// All PRs that have been responded to in some way.
    pub prs_with_responses: usize,
    /// All PRs that have been responded to officially.
    pub prs_with_official_responses: usize,
    /// How long does it take for someone to respond to a PR?
    pub pr_first_resp_time: Option<ResponseTimes>,
    /// How long does it take for an "official" response to a PR?
    pub pr_official_first_resp_time: Option<ResponseTimes>,
    /// How many PRs were merged?
    pub prs_merged: usize,
    /// The count of all PRs which were closed with being merged.
    pub prs_closed_without_merging: usize,
    /// How long does it take for PRs to be merged?
    pub pr_merge_time: Option<ResponseTimes>,
}

impl Statistics {
    pub fn report(self, repo: &str, commits: bool) -> String {
        let issues = if self.all_issues == 0 {
            "No issues found.".to_string()
        } else {
            let (any_median, any_mean) = self
                .issue_first_resp_time
                .map(|rt| (rt.median_time(), rt.average_time()))
                .unwrap_or_else(|| ("None".to_string(), "None".to_string()));

            let (official_median, official_mean) = self
                .issue_official_first_resp_time
                .map(|rt| (rt.median_time(), rt.average_time()))
                .unwrap_or_else(|| ("None".to_string(), "None".to_string()));

            format!(
                r#"
{} issues found, {} of which are now closed ({:.1}%).

- {} ({:.1}%) of these received a response.
- {} ({:.1}%) have an official response from a repo Owner or organization Member.

Response Times (any):
- Median: {}
- Average: {}

Response Times (official):
- Median: {}
- Average: {}"#,
                self.all_issues,
                self.all_closed_issues,
                percent(self.all_closed_issues, self.all_issues),
                self.issues_with_responses,
                percent(self.issues_with_responses, self.all_issues),
                self.issues_with_official_responses,
                percent(self.issues_with_official_responses, self.all_issues),
                any_median,
                any_mean,
                official_median,
                official_mean,
            )
        };

        let prs = if self.all_prs == 0 {
            "No Pull Requests found.".to_string()
        } else {
            let (any_median, any_mean) = self
                .pr_first_resp_time
                .map(|rt| (rt.median_time(), rt.average_time()))
                .unwrap_or_else(|| ("None".to_string(), "None".to_string()));

            let (official_median, official_mean) = self
                .pr_official_first_resp_time
                .map(|rt| (rt.median_time(), rt.average_time()))
                .unwrap_or_else(|| ("None".to_string(), "None".to_string()));

            let (merge_median, merge_mean) = self
                .pr_merge_time
                .map(|rt| (rt.median_time(), rt.average_time()))
                .unwrap_or_else(|| ("None".to_string(), "None".to_string()));

            format!(
                r#"
{} Pull Requests found, {} of which are now merged ({:.1}%).
{} have been closed without merging ({:.1}%).

- {} ({:.1}%) of these received a response.
- {} ({:.1}%) have an official response from a repo Owner or organization Member.

Response Times (any):
- Median: {}
- Average: {}

Response Times (official):
- Median: {}
- Average: {}

Time-to-Merge:
- Median: {}
- Average: {}"#,
                self.all_prs,
                self.prs_merged,
                percent(self.prs_merged, self.all_prs),
                self.prs_closed_without_merging,
                percent(self.prs_closed_without_merging, self.all_prs),
                self.prs_with_responses,
                percent(self.prs_with_responses, self.all_prs),
                self.prs_with_official_responses,
                percent(self.prs_with_official_responses, self.all_prs),
                any_median,
                any_mean,
                official_median,
                official_mean,
                merge_median,
                merge_mean,
            )
        };

        let contributors = format!(
            r#"
Top 10 Commentors (Issues and PRs):
{}

Top 10 Code Contributors (by merged PRs):
{}
"#,
            self.commentors
                .into_iter()
                .sorted_by(|a, b| b.1.cmp(&a.1))
                .take(10)
                .enumerate()
                .map(|(i, (name, issues))| format!("{:2}. {}: {}", i + 1, name, issues))
                .join("\n"),
            self.code_contributors
                .into_iter()
                .sorted_by(|a, b| b.1.cmp(&a.1))
                .take(10)
                .enumerate()
                .map(|(i, (name, prs))| format!("{:2}. {}: {}", i + 1, name, prs))
                .join("\n"),
        );

        let contributor_commits = if commits {
            format!(
                r#"
Top 10 Code Contributors (by commits-in-merged-PRs):
{}
"#,
                self.contributor_commits
                    .into_iter()
                    .sorted_by(|a, b| b.1.cmp(&a.1))
                    .take(10)
                    .enumerate()
                    .map(|(i, (name, commits))| format!("{:2}. {}: {}", i + 1, name, commits))
                    .join("\n"),
            )
        } else {
            "".to_string()
        };

        format!(
            r#"# Project Report for {}

## Issues
{}

## Pull Requests
{}

## Contributors
{}{}"#,
            repo, issues, prs, contributors, contributor_commits
        )
    }
}

/// Generate a client with preset headers for communicating with the Github API.
pub fn client(token: &str) -> anyhow::Result<HttpClient> {
    let client = HttpClient::builder()
        .default_header("Authorization", format!("bearer {}", token))
        .build()
        .context("Failed to create initial HTTP client.")?;

    Ok(client)
}

/// Given a repository name, look up the [`Thread`](struct.Thread.html)
/// statistics of all its Issues.
pub fn repo_threads(
    client: &HttpClient,
    ipb: &ProgressBar,
    ppb: &ProgressBar,
    serial: bool,
    commits: bool,
    start: &Option<DateTime<Utc>>,
    end: &Option<DateTime<Utc>>,
    owner: &str,
    repo: &str,
) -> anyhow::Result<Postings> {
    let i_msg = format!("Fetching Issues for {}/{}...", owner, repo);
    let p_msg = format!("Fetching Pull Requests for {}/{}...", owner, repo);

    let get_issues = || all_issues(client, start, end, owner, repo);
    let get_prs = || all_prs(client, start, end, commits, owner, repo);

    // Too much parallelism can trigger Github's abuse detection, so we offer
    // the "serial" option here.
    let (issues, prs) = if serial {
        let issues = with_progress(ipb, &i_msg, get_issues);
        let prs = with_progress(ppb, &p_msg, get_prs);
        (issues, prs)
    } else {
        rayon::join(
            || with_progress(ipb, &i_msg, get_issues),
            || with_progress(ppb, &p_msg, get_prs),
        )
    };

    Ok(Postings {
        issues: issues?,
        prs: prs?,
    })
}

/// Perform some action with an associated `ProgressBar`.
fn with_progress<F, A>(progress: &ProgressBar, msg: &str, f: F) -> A
where
    F: FnOnce() -> A,
{
    progress.enable_steady_tick(120);
    progress.set_message(&msg);
    let result = f();
    progress.finish_and_clear();
    result
}

fn all_issues(
    client: &HttpClient,
    start: &Option<DateTime<Utc>>,
    end: &Option<DateTime<Utc>>,
    owner: &str,
    repo: &str,
) -> anyhow::Result<Vec<Issue>> {
    repo::issues(client, end, &repo::Mode::Issues, owner, repo).map(|is| {
        is.into_iter()
            .filter(|i| {
                let after = start.map(|s| i.created_at >= s).unwrap_or(true);
                let before = end.map(|e| i.created_at <= e).unwrap_or(true);
                after && before
            })
            .map(|i| Issue(issue_thread(i)))
            .collect()
    })
}

fn all_prs(
    client: &HttpClient,
    start: &Option<DateTime<Utc>>,
    end: &Option<DateTime<Utc>>,
    commits: bool,
    owner: &str,
    repo: &str,
) -> anyhow::Result<Vec<PR>> {
    let mode = if commits {
        repo::Mode::PRsWithCommits
    } else {
        repo::Mode::PRs
    };
    repo::issues(client, end, &mode, owner, repo).map(|is| {
        is.into_iter()
            .filter(|i| {
                let after = start.map(|s| i.created_at >= s).unwrap_or(true);
                let before = end.map(|e| i.created_at <= e).unwrap_or(true);
                after && before
            })
            .map(|i| {
                let merged = i.merged_at;
                let commits = i.commits.as_ref().map(|cc| cc.total_count).unwrap_or(0);
                let thread = issue_thread(i);
                PR {
                    thread,
                    merged,
                    commits,
                }
            })
            .collect()
    })
}

fn ghost(author: &Option<repo::Author>) -> String {
    author
        .as_ref()
        .map(|a| a.login.clone())
        .unwrap_or_else(|| "@ghost".to_string())
}

fn issue_thread(issue: repo::Issue) -> Thread {
    let comments: Vec<repo::Comment> = issue.comments.edges.into_iter().map(|n| n.node).collect();

    // Need to be careful, since the first physical response might have been
    // from the Issue author.
    let first_comment = comments.iter().find(|c| !c.author_association.is_author());
    let first_responder = first_comment.map(|c| ghost(&c.author));
    let first_response = first_comment.map(|c| c.created_at);
    let first_official_response = comments
        .iter()
        .find(|c| c.author_association.is_official())
        .map(|c| c.created_at);
    let comment_counts = comments
        .iter()
        .map(|c| ghost(&c.author))
        .collect::<Counter<_>>()
        .into_map();

    Thread {
        author: ghost(&issue.author),
        posted: issue.created_at,
        closed: issue.closed_at,
        first_responder,
        first_response,
        first_official_response,
        comments: comment_counts,
    }
}

/// A curated list of the Top 100 users in a given location, ranked via their
/// contribution counts and weighted by followers.
pub fn user_contributions(client: &HttpClient, location: &str) -> anyhow::Result<UserContribs> {
    let total_users = contribs::user_count(client, location)?.user_count;
    let contributions = contribs::user_contributions(client, location)?
        .into_iter()
        .sorted_by(|a, b| b.contribs().cmp(&a.contribs()))
        .take(500)
        .sorted_by(|a, b| b.followers.total_count.cmp(&a.followers.total_count))
        .take(250)
        .sorted_by(|a, b| b.contribs().cmp(&a.contribs()))
        .take(100)
        .map(|uc| User::from(uc))
        .collect();

    Ok(UserContribs {
        total_users,
        contributions,
    })
}

fn hashmap_combine<K, V>(mut a: HashMap<K, V>, b: HashMap<K, V>) -> HashMap<K, V>
where
    K: Eq + std::hash::Hash,
    V: std::ops::AddAssign + Copy,
{
    for (k, v) in b {
        a.entry(k).and_modify(|e| *e += v).or_insert(v);
    }

    a
}

fn percent(a: usize, b: usize) -> f64 {
    100.0 * (a as f64) / (b as f64)
}

#[test]
fn hashmap_extend() {
    let mut first = HashMap::new();
    let mut second = HashMap::new();
    let message = "ABCDEF";

    for c in message.chars() {
        first.insert(c, 1);
        second.insert(c, 1);
    }

    let third = hashmap_combine(first, second);
    let elems: Vec<usize> = third.values().map(|v| *v).collect();

    assert_eq!(vec![2, 2, 2, 2, 2, 2], elems);
}
