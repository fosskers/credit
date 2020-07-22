use crate::github;
use anyhow::Context;
use indicatif::ProgressBar;
use isahc::prelude::*;
use serde::Deserialize;

/// The maximum number of results to fetch in a page.
const PAGE_SIZE: u32 = 5;

/// The maximum number of pages to pull when querying for user contributions.
const MAX_PAGES: u32 = 10 * (100 / PAGE_SIZE);

#[derive(Debug, Deserialize)]
struct SearchQuery {
    search: github::Paged<UserContributions>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserContributions {
    pub login: String,
    pub name: Option<String>,
    pub followers: Followers,
    pub contributions_collection: Contributions,
}

impl UserContributions {
    pub fn contributions(&self) -> u32 {
        let total = self
            .contributions_collection
            .contribution_calendar
            .total_contributions;
        total - self.contributions_collection.restricted_contributions_count
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Followers {
    pub total_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contributions {
    pub contribution_calendar: Calendar,
    pub restricted_contributions_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub total_contributions: u32,
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
                 followers {{ \
                   totalCount \
                 }} \
                 contributionsCollection {{ \
                   contributionCalendar {{ \
                     totalContributions \
                   }} \
                   restrictedContributionsCount \
                 }} \
               }} \
             }} \
           }} \
         }} \
       }}\" \
    }}",
        location,
        PAGE_SIZE,
        page.map(|p| format!(", after: \\\"{}\\\"", p))
            .unwrap_or_else(|| "".to_string()),
    )
}

/// Produce a list of Github Users, ordered by their contribution counts.
pub fn user_contributions(
    client: &HttpClient,
    location: &str,
) -> anyhow::Result<Vec<UserContributions>> {
    let bar = ProgressBar::new(MAX_PAGES as u64);
    bar.set_message("Fetching User contributions...");
    let result = user_contributions_work(client, &bar, location, None, 1);
    bar.finish_and_clear();
    result
}

fn user_contributions_work(
    client: &HttpClient,
    bar: &ProgressBar,
    location: &str,
    page: Option<&str>,
    page_num: u32,
) -> anyhow::Result<Vec<UserContributions>> {
    bar.inc(1);

    let body = users_query(location, page);

    let mut resp = client
        .post(github::V4_URL, body)
        .context("There was a problem calling the Github GraphQL API.")?;

    let text = resp.text()?;

    let result: github::Query<SearchQuery> = serde_json::from_str(&text)
        .with_context(|| format!("The response couldn't be decoded into JSON:\n{}", text))?;

    let page = result.data.search;
    let info = page.page_info;
    let mut users: Vec<UserContributions> = page.edges.into_iter().map(|n| n.node).collect();

    match info.end_cursor {
        Some(c) if info.has_next_page && page_num < MAX_PAGES => {
            let mut next = user_contributions_work(client, bar, location, Some(&c), page_num + 1)?;
            users.append(&mut next);
            Ok(users)
        }
        _ => Ok(users),
    }
}
