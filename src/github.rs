//! Github API types in reduced forms.

use anyhow::Context;
use curl::easy::{Easy, List};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::io::Read;

/// The never-changing URL to POST to for any V4 request.
const V4_URL: &str = "https://api.github.com/graphql";

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
#[derive(Deserialize)]
struct Query<A> {
    pub data: A,
}

/// Perform some generalized Github query.
pub fn lookup<A: DeserializeOwned>(token: &str, query: String) -> anyhow::Result<A> {
    let mut handle = Easy::new();
    let mut resp: Vec<u8> = Vec::new();
    handle.url(V4_URL)?;
    handle.fail_on_error(true)?;
    handle.post(true)?;
    handle.post_field_size(query.len() as u64)?;

    // --- Add Headers --- //
    let mut headers = List::new();
    headers.append(&format!("authorization: bearer {}", token))?;
    headers.append("user-agent: credit")?;
    handle.http_headers(headers)?;

    // Blocked off to allow `resp` to be borrowed immutably below.
    {
        let mut tx = handle.transfer();
        tx.read_function(move |buf| Ok(query.as_bytes().read(buf).unwrap_or(0)))?;
        tx.write_function(|data| {
            resp.extend_from_slice(data);
            Ok(data.len())
        })?;
        tx.perform()?;
    }

    let text = std::str::from_utf8(&resp)?;
    let result: Query<A> = serde_json::from_str(&text)
        .with_context(|| format!("The response couldn't be decoded into JSON:\n{}", text))?;

    Ok(result.data)
}
