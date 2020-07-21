//! Github API types in reduced forms. Only the fields that are useful to
//! `credit` are exposed.

use anyhow::Context;
use chrono::{DateTime, Utc};
use isahc::prelude::*;
use serde::{Deserialize, Serialize};

/// The maximum number of results to fetch in a page.
const CONTRIBUTION_PAGE_SIZE: u32 = 25;

/// The maximum number of pages to pull when querying for user contributions.
const CONTRIBUTION_MAX_PAGES: u32 = 10 * (100 / CONTRIBUTION_PAGE_SIZE);

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

#[derive(Debug, Deserialize)]
struct SearchQuery {
    search: Paged<UserContributions>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserContributions {
    pub login: String,
    pub name: String,
    pub contributions_collection: Contributions,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contributions {
    pub contribution_calendar: Calendar,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub total_contributions: u32,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
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

fn users_query(location: &str, page: Option<&str>) -> String {
    format!(
        "{{ \
         \"query\": \"{{ \
         search(type: USER, query: \\\"type:user location:{} sort:followers-desc\\\", first: {}{}) {{ \
           pageInfo {{ \
             hasNextPage \
             endCursor \
           }} \
           edges {{ \
             node {{ \
               ... on User {{ \
                 login \
                 name \
                 contributionsCollection {{ \
                   contributionCalendar {{ \
                     totalContributions \
                   }} \
                 }} \
               }} \
             }} \
           }} \
         }} \
       }}\" \
    }}",
        location,
        CONTRIBUTION_PAGE_SIZE,
        page.map(|p| format!(", after: \\\"{}\\\"", p))
            .unwrap_or_else(|| "".to_string()),
    )
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

    let text = resp.text()?;

    let issue_query: Query<IssueRepo> = serde_json::from_str(&text)
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

/// Discover the remaining API quota for the given token.
pub fn rate_limit(client: &HttpClient) -> anyhow::Result<RateLimit> {
    let mut resp = client
        .post(V4_URL, LIMIT_QUERY)
        .context("There was a problem calling the Github GraphQL API.")?;

    let text = resp.text()?;

    let limit_query: Query<RateLimitQuery> = serde_json::from_str(&text)
        .with_context(|| format!("The response couldn't be decoded into JSON:\n{}", text))?;

    Ok(limit_query.data.rate_limit)
}

/// Produce a list of Github Users, ordered by their contribution counts.
pub fn user_contributions(client: &HttpClient) -> anyhow::Result<Vec<UserContributions>> {
    user_contributions_work(client, None, 1)
}

fn user_contributions_work(
    client: &HttpClient,
    page: Option<&str>,
    page_num: u32,
) -> anyhow::Result<Vec<UserContributions>> {
    let body = users_query("Canada", page);

    let mut resp = client
        .post(V4_URL, body)
        .context("There was a problem calling the Github GraphQL API.")?;

    let text = resp.text()?;

    let result: Query<SearchQuery> = serde_json::from_str(&text)
        .with_context(|| format!("The response couldn't be decoded into JSON:\n{}", text))?;

    let page = result.data.search;
    let info = page.page_info;
    let mut users: Vec<UserContributions> = page.edges.into_iter().map(|n| n.node).collect();

    match info.end_cursor {
        Some(c) if info.has_next_page && page_num < CONTRIBUTION_MAX_PAGES => {
            let mut next = user_contributions_work(client, Some(&c), page_num + 1)?;
            users.append(&mut next);
            Ok(users)
        }
        _ => Ok(users),
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
