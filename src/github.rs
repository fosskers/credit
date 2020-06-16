//! Github API types in reduced forms. Only the fields that are useful to
//! `credit` are exposed.

use crate::error::Error;
use chrono::{DateTime, Utc};
use isahc::prelude::*;
use serde::Deserialize;

/// Some Github account.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Deserialize)]
pub struct User {
    pub login: String,
}

/// The bare minimum to parse the `pull_request` field of an Issue's JSON
/// response.
#[derive(Debug, Deserialize)]
pub struct PRField {
    pub url: String,
}

/// A reduced form of the full response of an Issue query.
#[derive(Debug, Deserialize)]
pub struct Issue {
    pub number: u32,
    pub state: String,
    pub user: User,
    pub comments: u32,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub pull_request: Option<PRField>,
}

/// A reduced form of the full response of a Pull Request query.
#[derive(Debug, Deserialize)]
pub struct PR {
    pub number: u32,
    pub state: String,
    pub user: User,
    pub comments: u32,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
    pub author_association: Association,
}

impl PR {
    /// Has this Pull Request been merged?
    pub fn is_merged(&self) -> bool {
        self.merged_at.is_some()
    }

    /// Was this Pull Request closed without merging?
    pub fn is_closed_not_merged(&self) -> bool {
        self.closed_at.is_some() && self.merged_at.is_none()
    }
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
pub fn all_issues(client: &HttpClient, owner: &str, repo: &str) -> Result<Vec<Issue>, Error> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/issues?state=all",
        owner, repo
    );

    let issues = client
        .get(url)?
        .json::<Vec<Issue>>()?
        .into_iter()
        .filter(|i| i.pull_request.is_some())
        .collect();

    Ok(issues)
}

/// All comments from a particular issue, in ascending order of posting date.
pub fn issue_comments(
    client: &HttpClient,
    owner: &str,
    repo: &str,
    issue: u32,
) -> Result<Vec<Comment>, Error> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/issues/{}/comments",
        owner, repo, issue
    );

    let comments = client.get(url)?.json()?;

    Ok(comments)
}

/// All Pull Requests belonging to a repository, regardless of status.
pub fn all_prs(client: &HttpClient, owner: &str, repo: &str) -> Result<Vec<PR>, Error> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls?state=all",
        owner, repo
    );

    let issues = client.get(url)?.json()?;

    Ok(issues)
}
