//! A library for measuring Github repository contributions.

mod github;

use anyhow::Context;
use chrono::{DateTime, Utc};
use counter::Counter;
use indicatif::ParallelProgressIterator;
use isahc::prelude::*;
use itertools::Itertools;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

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
    /// When, if ever, did an owner/member/collaborator/contributor first respond?
    pub first_contributor_response: Option<DateTime<Utc>>,
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
    /// Consumes the `Postings` to form all the statistics.
    pub fn statistics(self) -> Statistics {
        let all_issues = self.issues.len();

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

        let issues_with_contributor_responses = self
            .issues
            .iter()
            .filter_map(|i| i.0.first_contributor_response)
            .count();

        let issue_first_resp_time = self.resp_times(|| self.issues.iter(), |i| i.0.first_response);

        let issue_official_first_resp_time =
            self.resp_times(|| self.issues.iter(), |i| i.0.first_official_response);

        let issue_contributor_first_resp_time =
            self.resp_times(|| self.issues.iter(), |i| i.0.first_contributor_response);

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

        let prs_with_contributor_responses = self
            .prs
            .iter()
            .filter_map(|p| p.thread.first_contributor_response)
            .count();

        let pr_first_resp_time = self.resp_times(|| self.prs.iter(), |p| p.thread.first_response);

        let pr_official_first_resp_time =
            self.resp_times(|| self.prs.iter(), |p| p.thread.first_official_response);

        let pr_contributor_first_resp_time =
            self.resp_times(|| self.prs.iter(), |p| p.thread.first_contributor_response);

        let prs_closed_without_merging = self
            .prs
            .iter()
            .filter(|p| p.merged.is_none())
            .filter(|p| p.thread.closed.is_some())
            .count();

        let pr_merge_time = self.resp_times(|| self.prs.iter(), |p| p.merged);

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
                .fold(HashMap::new(), |acc, cs| hashmap_combine(acc, cs)),
            self.prs
                .into_iter()
                .map(|p| p.thread.comments)
                .fold(HashMap::new(), |acc, cs| hashmap_combine(acc, cs)),
        );

        Statistics {
            commentors,
            code_contributors,
            all_issues,
            issues_with_responses,
            issues_with_official_responses,
            issues_with_contributor_responses,
            issue_first_resp_time,
            issue_official_first_resp_time,
            issue_contributor_first_resp_time,
            all_prs,
            prs_closed_without_merging,
            prs_with_responses,
            prs_with_official_responses,
            prs_with_contributor_responses,
            pr_first_resp_time,
            pr_official_first_resp_time,
            pr_contributor_first_resp_time,
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
            let mean = resp_times
                .iter()
                .fold(chrono::Duration::seconds(0), |acc, x| acc + *x)
                .to_std()
                .ok()?;
            Some(ResponseTimes { median, mean })
        }
    }
}

/// Statistics involving [`Thread`](struct.Thread.html) response times.
#[derive(Debug, Serialize)]
pub struct ResponseTimes {
    pub median: Duration,
    pub mean: Duration,
    // pub std_deviation: f64,
}

/// Various compiled statistics regarding contributions to a Github repository.
///
/// For the relevant fields below, an "official" response is any made by a
/// repository Owner, an organization Member, or an invited Collaborator.
///
/// A "contributor" response is any made by the above three types or a
/// "Contributor" as marked by Github.
#[derive(Debug, Serialize)]
pub struct Statistics {
    /// All issue/PR commentors.
    pub commentors: HashMap<String, usize>,
    /// All users who had PRs merged.
    pub code_contributors: HashMap<String, usize>,
    /// The count of all issues, opened or closed.
    pub all_issues: usize,
    /// All issues that have been responded to in some way.
    pub issues_with_responses: usize,
    /// All issues that have been responded to "officially".
    pub issues_with_official_responses: usize,
    /// All issues that have been responded to by any contributor.
    pub issues_with_contributor_responses: usize,
    /// How long does it take for someone to respond to an issue?
    pub issue_first_resp_time: Option<ResponseTimes>,
    /// How long does it take for an "official" response?
    pub issue_official_first_resp_time: Option<ResponseTimes>,
    /// How long does it take for any contributor to respond to an
    /// issue?
    pub issue_contributor_first_resp_time: Option<ResponseTimes>,
    /// The count of all PRs, opened or closed.
    pub all_prs: usize,
    /// All PRs that have been responded to in some way.
    pub prs_with_responses: usize,
    /// All PRs that have been responded to officially.
    pub prs_with_official_responses: usize,
    /// All PRs that have been responded to by any contributor.
    pub prs_with_contributor_responses: usize,
    /// How long does it take for someone to respond to a PR?
    pub pr_first_resp_time: Option<ResponseTimes>,
    /// How long does it take for an "official" response to a PR?
    pub pr_official_first_resp_time: Option<ResponseTimes>,
    /// How long does it take for any contributor to respond to a PR?
    pub pr_contributor_first_resp_time: Option<ResponseTimes>,
    /// The count of all PRs which were closed with being merged.
    pub prs_closed_without_merging: usize,
    /// How long does it take for PRs to be merged?
    pub pr_merge_time: Option<ResponseTimes>,
}

/// Generate a client with preset headers for communicating with the Github API.
pub fn client(token: &str) -> anyhow::Result<HttpClient> {
    let client = HttpClient::builder()
        .default_header("Accept", "application/vnd.github.v3+json")
        .default_header("Authorization", format!("token {}", token))
        .build()
        .context("Failed to create initial HTTP client.")?;

    Ok(client)
}

/// Given a repository name, look up the [`Thread`](struct.Thread.html)
/// statistics of all its Issues.
pub fn repository_threads(
    client: &HttpClient,
    owner: &str,
    repo: &str,
) -> anyhow::Result<Postings> {
    println!("Fetching Issues...");
    let raw_issues = github::all_issues(client, owner, repo)?;
    println!("Fetching Issue comments...");
    let issues = raw_issues
        .par_iter()
        .progress_count(raw_issues.len() as u64)
        // Silently discards errors.
        .filter_map(|i| issue_thread(client, owner, repo, i).ok().map(Issue))
        .collect();

    println!("Fetching PRs...");
    let raw_prs = github::all_prs(client, owner, repo)?;
    println!("Fetching PR comments...");
    let prs = raw_prs
        .par_iter()
        .progress_count(raw_prs.len() as u64)
        // Sliently discards errors.
        .filter_map(|i| {
            issue_thread(client, owner, repo, i).ok().map(|t| PR {
                thread: t,
                merged: i.merged_at,
            })
        })
        .collect();

    Ok(Postings { issues, prs })
}

fn issue_thread(
    client: &HttpClient,
    owner: &str,
    repo: &str,
    issue: &github::Issue,
) -> anyhow::Result<Thread> {
    let comments = github::issue_comments(client, owner, repo, issue.number)?;

    // Need to be careful, since the first physical response might have been
    // from the Issue author.
    let first_comment = comments.iter().find(|c| !c.author_association.is_author());

    let first_responder = first_comment.map(|c| c.user.login.clone());
    let first_response = first_comment.map(|c| c.created_at);

    let first_official_response = comments
        .iter()
        .find(|c| c.author_association.is_official())
        .map(|c| c.created_at);

    let first_contributor_response = comments
        .iter()
        .find(|c| c.author_association.is_contributor())
        .map(|c| c.created_at);

    let mut comment_counts = HashMap::new();
    for c in comments {
        let counter = comment_counts.entry(c.user.login).or_insert(0);
        *counter += 1;
    }

    Ok(Thread {
        author: issue.user.login.clone(),
        posted: issue.created_at,
        closed: issue.closed_at,
        first_responder,
        first_response,
        first_official_response,
        first_contributor_response,
        comments: comment_counts,
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
