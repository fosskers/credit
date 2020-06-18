//! Github API types in reduced forms. Only the fields that are useful to
//! `credit` are exposed.

use anyhow::Context;
use chrono::{DateTime, Utc};
use isahc::prelude::*;
use serde::Deserialize;

/// Some Github account.
#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
}

/// The bare minimum to parse the `pull_request` field of an Issue's JSON
/// response.
#[derive(Debug, Deserialize)]
pub struct PRField {
    pub url: String,
}

/// A reduced form of the full response of an Issue or Pull Request query.
#[derive(Debug, Deserialize)]
pub struct Issue {
    pub number: u32,
    pub state: String,
    pub user: User,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
    pub pull_request: Option<PRField>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Association {
    Owner,
    Member,
    Collaborator,
    Contributor,
    Author,
    None,
}

impl Association {
    pub fn is_official(&self) -> bool {
        match self {
            Association::Owner => true,
            Association::Member => true,
            Association::Collaborator => true,
            _ => false,
        }
    }

    pub fn is_contributor(&self) -> bool {
        match self {
            Association::Owner => true,
            Association::Member => true,
            Association::Collaborator => true,
            Association::Contributor => true,
            _ => false,
        }
    }

    pub fn is_author(&self) -> bool {
        match self {
            Association::Author => true,
            _ => false,
        }
    }
}

/// An issue comment.
#[derive(Debug, Deserialize)]
pub struct Comment {
    pub user: User,
    pub created_at: DateTime<Utc>,
    pub author_association: Association,
}

/// All issues belonging to a repository.
///
/// From the notes of the Github API:
///
/// > GitHub's REST API v3 considers every pull request an issue, but not every
/// > issue is a pull request.
pub fn all_issues(client: &HttpClient, owner: &str, repo: &str) -> anyhow::Result<Vec<Issue>> {
    all_issues_paged(client, owner, repo, 1)
}

pub fn all_issues_paged(
    client: &HttpClient,
    owner: &str,
    repo: &str,
    page: u32,
) -> anyhow::Result<Vec<Issue>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/issues?state=all&page={}&per_page=100",
        owner, repo, page
    );

    let mut resp = client
        .get(url)
        .context("There was a problem fetching Issue data.")?;

    let mut issues = resp
        .json::<Vec<Issue>>()
        .context("The issue response couldn't be decoded into JSON.")?
        .into_iter()
        .filter(|i| i.pull_request.is_none())
        .collect();

    match resp
        .headers()
        .get("link")
        .and_then(|l| l.to_str().ok())
        .and_then(|l| parse_link_header::parse(l).ok())
        .and_then(|mut link_map| link_map.remove(&Some("next".to_string())))
    {
        None => Ok(issues),
        Some(_) => {
            let mut next = all_issues_paged(client, owner, repo, page + 1)?;
            issues.append(&mut next);
            Ok(issues)
        }
    }
}

/// All comments from a particular issue, in ascending order of posting date.
pub fn issue_comments(
    client: &HttpClient,
    owner: &str,
    repo: &str,
    issue: u32,
) -> anyhow::Result<Vec<Comment>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/issues/{}/comments?per_page=100",
        owner, repo, issue
    );

    let comments = client
        .get(url)
        .context("There was a problem fetching comments.")?
        .json()
        .context("The comments couldn't be decoded into JSON.")?;

    Ok(comments)
}

/// All Pull Requests belonging to a repository, regardless of status.
pub fn all_prs(client: &HttpClient, owner: &str, repo: &str) -> anyhow::Result<Vec<Issue>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls?state=all&per_page=100",
        owner, repo
    );

    let prs = client
        .get(url)
        .context("There was a problem fetching PR data.")?
        .json()
        .context("The PR response couldn't be decoded into JSON.")?;

    Ok(prs)
}
