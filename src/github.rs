//! Github API types in reduced forms. Only the fields that are useful to
//! `credit` are exposed.

use anyhow::Context;
use chrono::{DateTime, Utc};
use isahc::prelude::*;
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
pub struct Query<T> {
    pub data: T,
}
