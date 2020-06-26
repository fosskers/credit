//! Github API types in reduced forms. Only the fields that are useful to
//! `credit` are exposed.

use anyhow::Context;
use chrono::{DateTime, Utc};
use isahc::prelude::*;
use serde::de::DeserializeOwned;
use serde::Deserialize;

// GRAPHQL NOTES
//
// I won't bother paginating on the comments, which means that only the first
// 100 comments per issue will be counted.
//
// Much less likely to hit any rate limits via the GraphQL API.
//
// Orders of magnitude fewer HTTP calls necessary to get all data.
//
// Issue and PR data is NOT mixed! So no need to refilter out the PRs like I'm
// currently doing.

/// The never-changing URL to POST to for any V4 request.
const V4_URL: &str = "https://api.github.com/graphql";

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueV4 {
    pub author: Option<Author>,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub comments: Edges<CommentV4>,
}

#[derive(Debug, Deserialize)]
pub struct Author {
    pub login: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentV4 {
    pub author: Option<Author>,
    pub author_association: Association,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct IssueQuery {
    data: IssueRepo,
}

#[derive(Deserialize)]
struct IssueRepo {
    repository: Issues,
}

#[derive(Deserialize)]
struct Issues {
    issues: Paged<IssueV4>,
}

// TODO Call 100 of each.
pub fn issue_query(owner: &str, repo: &str, page: Option<&str>) -> String {
    format!(
        "{{ \
    \"query\": \"{{ \
        repository(owner: \\\"{}\\\", name: \\\"{}\\\") {{ \
            issues(first: 100{}) {{ \
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
        page.map(|p| format!(", after: \\\"{}\\\"", p))
            .unwrap_or_else(|| "".to_string())
    )
}

pub fn v4_issues(client: &HttpClient, owner: &str, repo: &str) -> anyhow::Result<Vec<IssueV4>> {
    v4_issues_work(client, owner, repo, None)
}

// TODO Generalize to be reusable by both Issues and PRs.
fn v4_issues_work(
    client: &HttpClient,
    owner: &str,
    repo: &str,
    page: Option<&str>,
) -> anyhow::Result<Vec<IssueV4>> {
    let body = issue_query(owner, repo, page);

    let mut resp = client
        .post(V4_URL, body)
        .context("There was a problem calling the Github GraphQL API.")?;

    let issue_query: IssueQuery = resp
        .json()
        .context("The responses couldn't be decoded into JSON.")?;

    let mut issues: Vec<IssueV4> = issue_query
        .data
        .repository
        .issues
        .edges
        .into_iter()
        .map(|n| n.node)
        .collect();

    let info = issue_query.data.repository.issues.page_info;

    match info.end_cursor {
        Some(c) if info.has_next_page => {
            let mut next = v4_issues_work(client, owner, repo, Some(&c))?;
            issues.append(&mut next);
            Ok(issues)
        }
        _ => Ok(issues),
    }
}

// {
//     repository(owner: "fosskers", name: "aura") {
//         pullRequests(first: 100) {
//             pageInfo {
//                 hasNextPage
//                     endCursor
//             }
//             edges {
//                 node {
//                     author {
//                         login
//                     }
//                     createdAt
//                         closedAt
//                         mergedAt
//                         comments(first: 100) {
//                             edges {
//                                 node {
//                                     author {
//                                         login
//                                     }
//                                     authorAssociation
//                                         createdAt
//                                 }
//                             }
//                         }
//                 }
//             }
//         }
//     }
// }

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
    let url = format!(
        "https://api.github.com/repos/{}/{}/issues?state=all&per_page=100",
        owner, repo,
    );

    let issues: Vec<Issue> = paged_lookups(client, &url)?;

    Ok(issues
        .into_iter()
        .filter(|i| i.pull_request.is_none())
        .collect())
}

// Could use `rayon` here to parallelize over all the queries that are known to
// be necessary. If the first ever response had a `link` header, it'll also
// include the `last` ref. From that we could just do them all at the same time
// with "guessed" URLs (although Github says not to guess.)
/// Query the Github API continually until the `link` header claims there aren't
/// any further pages.
fn paged_lookups<A>(client: &HttpClient, url: &str) -> anyhow::Result<Vec<A>>
where
    A: DeserializeOwned,
{
    let mut resp = client
        .get(url)
        .context("There was a problem calling the Github API.")?;

    let mut results = resp
        .json::<Vec<A>>()
        .context("The responses couldn't be decoded into JSON.")?;

    match resp
        .headers()
        .get("link")
        .and_then(|l| l.to_str().ok())
        .and_then(|l| parse_link_header::parse(l).ok())
        .and_then(|mut link_map| link_map.remove(&Some("next".to_string())))
    {
        None => Ok(results),
        Some(link) => {
            let mut next = paged_lookups(client, &link.raw_uri)?;
            results.append(&mut next);
            Ok(results)
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

    paged_lookups(client, &url)
}

/// All Pull Requests belonging to a repository, regardless of status.
pub fn all_prs(client: &HttpClient, owner: &str, repo: &str) -> anyhow::Result<Vec<Issue>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls?state=all&per_page=100",
        owner, repo
    );

    paged_lookups(client, &url)
}
