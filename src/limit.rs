//! Types and functions for the `limit` command.

use crate::github;
use anyhow::Context;
use chrono::{DateTime, Utc};
use isahc::prelude::*;
use serde::{Deserialize, Serialize};

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

/// Discover the remaining API quota for the given token.
pub fn rate_limit(client: &HttpClient) -> anyhow::Result<RateLimit> {
    let mut resp = client
        .post(github::V4_URL, LIMIT_QUERY)
        .context("There was a problem calling the Github GraphQL API.")?;

    let text = resp.text()?;

    let limit_query: github::Query<RateLimitQuery> = serde_json::from_str(&text)
        .with_context(|| format!("The response couldn't be decoded into JSON:\n{}", text))?;

    Ok(limit_query.data.rate_limit)
}
