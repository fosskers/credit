//! Types and functions for the `limit` command.

use crate::github;
use chrono::{DateTime, Utc};
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
pub fn rate_limit() -> anyhow::Result<RateLimit> {
    let result: RateLimitQuery = github::lookup(LIMIT_QUERY.to_string())?;
    Ok(result.rate_limit)
}
