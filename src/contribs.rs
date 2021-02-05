//! Types and functions for the `users` command.

use crate::github;
use indicatif::ProgressBar;
use serde::Deserialize;
use std::thread;
use std::time::Duration;

/// The maximum number of results to fetch in a page.
const PAGE_SIZE: u32 = 5;

/// The maximum number of pages to pull when querying for user contributions.
const MAX_PAGES: u32 = 10 * (100 / PAGE_SIZE);

/// Only attempt to query for a page this many times.
const MAX_ATTEMPTS: u32 = 10;

#[derive(Deserialize)]
struct SearchQuery {
    search: github::Paged<UserContribs>,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Followers {
    pub total_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contributions {
    pub contribution_calendar: Calendar,
    pub restricted_contributions_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub total_contributions: u32,
}

#[derive(Deserialize)]
pub struct UserCountQuery {
    pub search: UserCount,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCount {
    pub user_count: u32,
}

fn user_count_query(location: &str) -> String {
    format!(
        "{{ \
         \"query\": \"{{ \
             search(type: USER, query: \\\"type:user location:{}\\\") {{ \
               userCount \
             }} \
           }}\" \
         }}",
        location,
    )
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

/// How many users claim to be from a certain area?
pub fn user_count(token: &str, location: &str) -> anyhow::Result<UserCount> {
    let body = user_count_query(location);
    let result: UserCountQuery = github::lookup(token, body)?;
    Ok(result.search)
}

/// Produce a list of Github Users, ordered by their contribution counts.
pub fn user_contributions(token: &str, location: &str) -> anyhow::Result<Vec<UserContribs>> {
    eprintln!("Fetching data pages from Github...");
    let progress = ProgressBar::new(MAX_PAGES as u64);
    let result = user_contributions_work(token, &progress, location, None, 1, 1);
    progress.finish_and_clear();
    result
}

fn user_contributions_work(
    token: &str,
    progress: &ProgressBar,
    location: &str,
    page: Option<&str>,
    page_num: u32,
    attempts: u32,
) -> anyhow::Result<Vec<UserContribs>> {
    let body = users_query(location, page);
    match github::lookup::<SearchQuery>(token, body) {
        Err(_) if attempts < MAX_ATTEMPTS => {
            thread::sleep(Duration::from_secs(10));
            user_contributions_work(token, progress, location, page, page_num, attempts + 1)
        }
        Err(e) => Err(e),
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
                        token,
                        progress,
                        location,
                        Some(&c),
                        page_num + 1,
                        1,
                    )?;
                    users.append(&mut next);
                    Ok(users)
                }
                _ => Ok(users),
            }
        }
    }
}
