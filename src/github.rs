//! Github API types in reduced forms.

use anyhow::Context;
use isahc::prelude::*;
use serde::de::DeserializeOwned;
use serde::Deserialize;

/// The never-changing URL to POST to for any V4 request.
pub const V4_URL: &str = "https://api.github.com/graphql";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
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
pub struct Paged<A> {
    pub page_info: PageInfo,
    pub edges: Vec<Node<A>>,
}

/// The top-level results of a GraphQL query.
#[derive(Debug, Deserialize)]
pub struct Query<A> {
    pub data: A,
}

/// Perform some generalized Github query.
pub fn lookup<A>(client: &HttpClient, query: String) -> anyhow::Result<A>
where
    A: DeserializeOwned,
{
    let mut resp = client
        .post(V4_URL, query)
        .context("There was a problem calling the Github GraphQL API.")?;

    let text = resp.text()?;

    let result: Query<A> = serde_json::from_str(&text)
        .with_context(|| format!("The response couldn't be decoded into JSON:\n{}", text))?;

    Ok(result.data)
}
