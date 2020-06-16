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
pub struct PR {
    pub url: String,
}

/// A reduced form of the full response of an issue query.
#[derive(Debug, Deserialize)]
pub struct Issue {
    pub number: u32,
    pub user: User,
    pub comments: u32,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub pull_request: Option<PR>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Association {
    Owner,
    Member,
    Collaborator,
    Contributor,
    Author,
}

impl Association {
    pub fn is_owner(&self) -> bool {
        match self {
            Association::Owner => true,
            _ => false,
        }
    }

    pub fn is_contributor(&self) -> bool {
        match self {
            Association::Contributor => true,
            _ => false,
        }
    }
}

/// An issue comment.
#[derive(Debug, Deserialize)]
pub struct Comment {
    pub user: User,
    pub created_at: DateTime<Utc>,
    pub author_association: Option<Association>,
}

/// All issues and PRs belonging to a repository.
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

    let issues = client.get(url)?.json()?;

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
