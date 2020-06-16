//! A library for measuring Github repository contributions.

pub mod error;
mod github;

use chrono::{DateTime, Utc};
use error::Error;
use isahc::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

/// A Github Issue.
#[derive(Debug)]
pub struct Issue(Thread);

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
    pub official_first_response: Option<DateTime<Utc>>,
    /// When, if ever, did an owner/member/collaborator/contributor first respond?
    pub contributor_first_response: Option<DateTime<Utc>>,
    /// Comment counts of everyone who participated.
    pub comments: HashMap<String, u32>,
}

/// A collection of Issue and Pull Request [`Thread`](struct.Thread.html)s.
#[derive(Debug)]
pub struct Postings {
    pub issues: Vec<Issue>,
    pub prs: Vec<PR>,
}

impl Postings {
    // pub fn statistics(self) -> Statistics {
    //     Statistics {
    //         commentors: HashMap::new(),
    //         code_contributors: HashMap::new(),
    //         all_issues: 0,
    //         issues_with_responses
    //     }
    // }
}

/// Statistics involving [`Thread`](struct.Thread.html) response times.
pub struct ResponseTimes {
    pub median_response_time: Duration,
    pub mean_response_time: Duration,
    pub std_deviation: f64,
}

/// Various compiled statistics regarding contributions to a Github repository.
///
/// For the relevant fields below, an "official" response is any made by a
/// repository Owner, an organization Member, or an invited Collaborator.
///
/// A "contributor" response is any made by the above three types or a
/// "Contributor" as marked by Github.
pub struct Statistics {
    /// All issue/PR commentors.
    pub commentors: HashMap<String, u32>,
    /// All users who had PRs merged.
    pub code_contributors: HashMap<String, u32>,
    /// The count of all issues, opened or closed.
    pub all_issues: u32,
    /// All issues that have been responded to in some way.
    pub issues_with_responses: u32,
    /// All issues that have been responded to "officially".
    pub issues_with_official_responses: u32,
    /// All issues that have been responded to by any contributor.
    pub issues_with_contributor_responses: u32,
    /// How long does it take for someone to respond to an issue?
    pub issue_first_resp_time: ResponseTimes,
    /// How long does it take for an "official" response?
    pub issue_official_first_resp_time: ResponseTimes,
    /// How long does it take for any contributor to respond to an
    /// issue?
    pub issue_contributor_first_resp_time: ResponseTimes,
    /// The count of all PRs, opened or closed.
    pub all_prs: u32,
    /// All PRs that have been responded to in some way.
    pub prs_with_responses: u32,
    /// All PRs that have been responded to officially.
    pub prs_with_official_responses: u32,
    /// All PRs that have been responded to by any contributor.
    pub prs_with_contributor_responses: u32,
    /// How long does it take for someone to respond to a PR?
    pub pr_first_resp_time: ResponseTimes,
    /// How long does it take for an "official" response to a PR?
    pub pr_official_first_resp_time: ResponseTimes,
    /// How long does it take for any contributor to respond to a PR?
    pub pr_contributor_first_resp_time: ResponseTimes,
}

/// Generate a client with preset headers for communicating with the Github API.
pub fn client(token: &str) -> Result<HttpClient, Error> {
    let client = HttpClient::builder()
        .default_header("Accept", "application/vnd.github.v3+json")
        .default_header("Authorization", format!("token {}", token))
        .build()?;

    Ok(client)
}

// TODO Use a progress bar here.
/// Given a repository name, look up the [`Thread`](struct.Thread.html)
/// statistics of all its Issues.
pub fn repository_threads(client: &HttpClient, owner: &str, repo: &str) -> Result<Postings, Error> {
    let issues = github::all_issues(client, owner, repo)?
        .par_iter()
        // TODO Handle errors better!
        .filter_map(|i| match issue_thread(client, owner, repo, i) {
            Ok(t) => Some(Issue(t)),
            Err(e) => {
                eprintln!("ISSUE PROBLEM: {:?}", e);
                None
            }
        })
        .collect();

    let prs = github::all_prs(client, owner, repo)?
        .par_iter()
        // TODO Handle errors better!
        .filter_map(|i| match issue_thread(client, owner, repo, i) {
            Ok(t) => Some(PR {
                thread: t,
                merged: i.merged_at,
            }),
            Err(e) => {
                eprintln!("PR PROBLEM: {:?}", e);
                None
            }
        })
        .collect();

    Ok(Postings { issues, prs })
}

fn issue_thread(
    client: &HttpClient,
    owner: &str,
    repo: &str,
    issue: &github::Issue,
) -> Result<Thread, Error> {
    let comments = github::issue_comments(client, owner, repo, issue.number)?;

    // Need to be careful, since the first physical response might have been
    // from the Issue author.
    let first_comment = comments.iter().find(|c| !c.author_association.is_author());

    let first_responder = first_comment.map(|c| c.user.login.clone());
    let first_response = first_comment.map(|c| c.created_at);

    let official_first_response = comments
        .iter()
        .find(|c| c.author_association.is_official())
        .map(|c| c.created_at);

    let contributor_first_response = comments
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
        official_first_response,
        contributor_first_response,
        comments: comment_counts,
    })
}

// Pagination notes: https://developer.github.com/v3/#pagination
// - Can ask for 100 items per page.
