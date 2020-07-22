//! Types and functions for the `users` command.

use crate::github;
use indicatif::ProgressBar;
use isahc::prelude::*;
use serde::Deserialize;
use std::thread;
use std::time::Duration;

/// The maximum number of results to fetch in a page.
const PAGE_SIZE: u32 = 10;

/// The maximum number of pages to pull when querying for user contributions.
const MAX_PAGES: u32 = 10 * (100 / PAGE_SIZE);

#[derive(Debug, Deserialize)]
struct SearchQuery {
    search: github::Paged<UserContribs>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserContribs {
    pub login: String,
    pub name: Option<String>,
    pub followers: Followers,
    pub contributions_collection: Contributions,
}

impl UserContribs {
    pub fn contribs(&self) -> u32 {
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
) -> anyhow::Result<Vec<UserContribs>> {
    let progress = ProgressBar::new(MAX_PAGES as u64);
    let result = user_contributions_work(client, &progress, location, None, 1);
    progress.finish_and_clear();
    result
}

fn user_contributions_work(
    client: &HttpClient,
    progress: &ProgressBar,
    location: &str,
    page: Option<&str>,
    page_num: u32,
) -> anyhow::Result<Vec<UserContribs>> {
    let body = users_query(location, page);
    match github::lookup::<SearchQuery>(client, body) {
        Err(_) => {
            thread::sleep(Duration::from_secs(10));
            user_contributions_work(client, progress, location, page, page_num)
        }
        Ok(result) => {
            progress.inc(1);
            let page = result.search;
            let info = page.page_info;
            let mut users: Vec<UserContribs> = page.edges.into_iter().map(|n| n.node).collect();

            match info.end_cursor {
                // Ends early if we've found users with 0 followers.
                Some(c)
                    if info.has_next_page
                        && page_num < MAX_PAGES
                        && users
                            .last()
                            .map(|uc| uc.followers.total_count > 0)
                            .unwrap_or(false) =>
                {
                    let mut next = user_contributions_work(
                        client,
                        progress,
                        location,
                        Some(&c),
                        page_num + 1,
                    )?;
                    users.append(&mut next);
                    Ok(users)
                }
                _ => Ok(users),
            }
        }
    }
}
