//! Types and functions for the `repo` command.

use crate::github;
use anyhow::Context;
use chrono::{DateTime, Utc};
use isahc::prelude::*;
use serde::Deserialize;

/// A single structure that represents the results from either an `issues` call
/// or a `pullRequests` call from the GraphQL API.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub author: Option<Author>,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
    pub comments: github::Edges<Comment>,
    pub commits: Option<CommitCount>,
}

#[derive(Debug, Deserialize)]
pub struct Author {
    pub login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub author: Option<Author>,
    pub author_association: Association,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitCount {
    pub total_count: usize,
}

#[derive(Deserialize)]
struct IssueRepo {
    repository: Issues,
}

#[derive(Deserialize)]
#[serde(untagged)] // Serde is so good.
enum Issues {
    Issue {
        issues: github::Paged<Issue>,
    },
    #[serde(rename_all = "camelCase")]
    PullRequest {
        pull_requests: github::Paged<Issue>,
    },
}

impl Issues {
    fn page(self) -> github::Paged<Issue> {
        match self {
            Issues::Issue { issues } => issues,
            Issues::PullRequest { pull_requests } => pull_requests,
        }
    }
}

pub enum Mode {
    Issues,
    PRs,
    PRsWithCommits,
}

impl Mode {
    fn graph_call(&self) -> &str {
        match self {
            Mode::Issues => "issues",
            _ => "pullRequests",
        }
    }

    fn merged_field(&self) -> &str {
        match self {
            Mode::Issues => "",
            _ => "mergedAt",
        }
    }

    fn commits(&self) -> &str {
        match self {
            Mode::PRsWithCommits => "commits { totalCount }",
            _ => "",
        }
    }
}

fn issue_query(mode: &Mode, owner: &str, repo: &str, page: Option<&str>) -> String {
    format!(
        "{{ \
    \"query\": \"{{ \
        repository(owner: \\\"{}\\\", name: \\\"{}\\\") {{ \
            {}(first: 100{}) {{ \
                pageInfo {{ \
                    hasNextPage \
                    endCursor \
                }} \
                edges {{ \
                    node {{ \
                        author {{ \
                            login \
                        }} \
                        createdAt \
                        closedAt \
                        {} \
                        {} \
                        comments(first: 100) {{ \
                            edges {{ \
                                node {{ \
                                    author {{ \
                                        login \
                                    }} \
                                    authorAssociation \
                                    createdAt \
                                }} \
                            }} \
                        }} \
                    }} \
                }} \
            }} \
        }} \
    }}\" \
    }}",
        owner,
        repo,
        mode.graph_call(),
        page.map(|p| format!(", after: \\\"{}\\\"", p))
            .unwrap_or_else(|| "".to_string()),
        mode.merged_field(),
        mode.commits(),
    )
}

/// Fetch all Issues or Pull Requests for a project, depending on the `Mode` given.
pub fn issues(
    client: &HttpClient,
    end: &Option<DateTime<Utc>>,
    mode: &Mode,
    owner: &str,
    repo: &str,
) -> anyhow::Result<Vec<Issue>> {
    issues_work(client, end, mode, owner, repo, None)
}

fn issues_work(
    client: &HttpClient,
    end: &Option<DateTime<Utc>>,
    mode: &Mode,
    owner: &str,
    repo: &str,
    page: Option<&str>,
) -> anyhow::Result<Vec<Issue>> {
    let body = issue_query(mode, owner, repo, page);

    let mut resp = client
        .post(github::V4_URL, body)
        .context("There was a problem calling the Github GraphQL API.")?;

    let text = resp.text()?;

    let issue_query: github::Query<IssueRepo> = serde_json::from_str(&text)
        .with_context(|| format!("The response couldn't be decoded into JSON:\n{}", text))?;

    let page = issue_query.data.repository.page();
    let info = page.page_info;
    let mut issues: Vec<Issue> = page.edges.into_iter().map(|n| n.node).collect();

    // If the user supplied `--end`, we don't need to page past the point
    // they're looking for.
    let stop_early = end
        .and_then(|e| issues.last().map(|i| i.created_at > e))
        .unwrap_or(false);

    match info.end_cursor {
        Some(c) if info.has_next_page && !stop_early => {
            let mut next = issues_work(client, end, mode, owner, repo, Some(&c))?;
            issues.append(&mut next);
            Ok(issues)
        }
        _ => Ok(issues),
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

    pub fn is_author(&self) -> bool {
        match self {
            Association::Author => true,
            _ => false,
        }
    }
}