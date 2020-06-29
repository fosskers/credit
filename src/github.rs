//! Github API types in reduced forms. Only the fields that are useful to
//! `credit` are exposed.

use anyhow::Context;
use chrono::{DateTime, Utc};
use isahc::prelude::*;
use serde::{Deserialize, Serialize};

/// The never-changing URL to POST to for any V4 request.
const V4_URL: &str = "https://api.github.com/graphql";

const LIMIT_QUERY: &str = "{ \
  \"query\": \"{ \
      rateLimit { \
        limit \
        remaining \
        resetAt \
      } \
  }\" \
}";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub limit: u32,
    pub remaining: u32,
    pub reset_at: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RateLimitQuery {
    rate_limit: RateLimit,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Edges<A> {
    pub edges: Vec<Node<A>>,
}

#[derive(Debug, Deserialize)]
pub struct Node<A> {
    pub node: A,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Paged<A> {
    page_info: PageInfo,
    edges: Vec<Node<A>>,
}

/// A single structure that represents the results from either an `issues` call
/// or a `pullRequests` call from the GraphQL API.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub author: Option<Author>,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
    pub comments: Edges<Comment>,
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

/// The top-level results of a GraphQL query.
#[derive(Deserialize)]
struct Query<T> {
    data: T,
}

#[derive(Deserialize)]
struct IssueRepo {
    repository: Issues,
}

#[derive(Deserialize)]
#[serde(untagged)] // Serde is so good.
enum Issues {
    Issue {
        issues: Paged<Issue>,
    },
    #[serde(rename_all = "camelCase")]
    PullRequest {
        pull_requests: Paged<Issue>,
    },
}

impl Issues {
    fn page(self) -> Paged<Issue> {
        match self {
            Issues::Issue { issues } => issues,
            Issues::PullRequest { pull_requests } => pull_requests,
        }
    }
}

pub enum Mode {
    Issues,
    PRs,
}

impl Mode {
    fn graph_call(&self) -> &str {
        match self {
            Mode::Issues => "issues",
            Mode::PRs => "pullRequests",
        }
    }

    fn merged_field(&self) -> &str {
        match self {
            Mode::Issues => "",
            Mode::PRs => "mergedAt",
        }
    }

    fn commits(&self) -> &str {
        match self {
            Mode::Issues => "",
            Mode::PRs => "commits { totalCount }",
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
        .post(V4_URL, body)
        .context("There was a problem calling the Github GraphQL API.")?;

    let issue_query: Query<IssueRepo> = resp
        .json()
        .context("The responses couldn't be decoded into JSON.")?;

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

/// Discover the remaining API quota for the given token.
pub fn rate_limit(client: &HttpClient) -> anyhow::Result<RateLimit> {
    let mut resp = client
        .post(V4_URL, LIMIT_QUERY)
        .context("There was a problem calling the Github GraphQL API.")?;

    let limit_query: Query<RateLimitQuery> = resp
        .json()
        .context("The response couldn't be decoded into JSON.")?;

    Ok(limit_query.data.rate_limit)
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
