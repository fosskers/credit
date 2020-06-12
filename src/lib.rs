//! A library for measuring Github repository contributions.

use auto_from::From;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Duration;

/// Errors that occur during Github communication, etc.
#[derive(From)]
pub enum Error {
    Isahc(isahc::Error),
}

/// Some Github account.
#[derive(Debug)]
pub struct User(String);

/// A thread of conversation on Github.
///
/// This could either be associated with an Issue or a PR.
pub struct Thread {
    /// Who opened the thread?
    pub author: User,
    /// When was the thread opened?
    pub posted: DateTime<Utc>,
    /// If it's already closed, when was it?
    pub closed: Option<DateTime<Utc>>,
    /// Who responded first?
    pub first_responser: User,
    /// When, if ever, did a repo owner first respond?
    pub owner_first_response: Option<DateTime<Utc>>,
    /// Comment counts of everyone who participated.
    pub comments: HashMap<User, u32>,
}

/// Various compiled statistics regarding contributions to a Github repository.
pub struct Statistics {
    /// All issue/PR commentors.
    pub commentors: HashMap<User, u32>,
    /// All users who had PRs merged.
    pub code_contributors: HashMap<User, u32>,
    /// The count of all issues, opened or closed.
    pub all_issues: u32,
    /// All issues that have been responded to in some way.
    pub issues_with_responses: u32,
    /// All issues that have been responded to by a repo owner.
    pub issues_with_owner_responses: u32,
    /// On average, how long does it take for someone to respond to an issue?
    pub issue_avg_first_resp_time: Duration,
    /// On average, how long does it take for a repo owner to respond to an
    /// issue?
    pub issue_avg_owner_first_resp_time: Duration,
    /// The count of all PRs, opened or closed.
    pub all_prs: u32,
    /// All PRs that have been responded to in some way.
    pub prs_with_responses: u32,
    /// All PRs that have been responded to by a repo owner.
    pub prs_with_owner_responses: u32,
    /// On average, how long does it take for someone to respond to a PR?
    pub pr_avg_first_resp_time: Duration,
    /// On average, how long does it take for a repo owner to respond to a PR?
    pub pr_avg_owner_first_resp_time: Duration,
}

/// Given a repository name, look up the [`Thread`](struct.Thread.html)
/// statistics of all its Issues.
pub fn repository_issues(_: &str) -> Result<Vec<Thread>, Error> {
    Ok(vec![])
}

/// Given a repository name, look up the [`Thread`](struct.Thread.html)
/// statistics of all its PRs.
pub fn repository_prs(_: &str) -> Result<Vec<Thread>, Error> {
    Ok(vec![])
}

/// Who are the owners of the given repository?
pub fn repository_owners(_: &str) -> Result<Vec<User>, Error> {
    let resp = isahc::get("https://www.fosskers.ca")?;
    println!("{:#?}", resp);

    Ok(vec![])
}
