//! A tool for measuring repository contributions.

use anyhow::anyhow;
use gumdrop::{Options, ParsingStyle};
use itertools::Itertools;
use rayon::prelude::*;
use std::process;

/// A tool for measuring repository contributions.
#[derive(Options)]
struct Args {
    /// Print this help text.
    help: bool,

    /// Command to perform.
    #[options(command)]
    command: Option<Command>,
}

#[derive(Options)]
enum Command {
    /// Analyse repository contributions.
    Repo(Repo),
    /// Check the Github API for remaining rate limit allowance.
    Limit(Limit),
}

/// Analyse repository contributions.
#[derive(Options)]
struct Repo {
    /// Print this help text.
    help: bool,

    /// Output as JSON.
    json: bool,

    /// Github personal access token.
    token: String,

    /// A Github repository to check (can pass multiple times).
    #[options(free, parse(try_from_str = "split_repo"))]
    repos: Vec<(String, String)>,
}

/// Check the Github API for remaining rate limit allowance.
#[derive(Options)]
struct Limit {
    /// Print this help text.
    help: bool,

    /// Github personal access token.
    token: String,
}

fn main() {
    let args = Args::parse_args_or_exit(ParsingStyle::AllOptions);

    match args.command {
        None => report(Err(anyhow!(
            "No command specified. Did you mean to use `repo`?"
        ))),
        Some(Command::Limit(l)) => report(limit(l)),
        Some(Command::Repo(r)) => report(repo(r)),
    }
}

/// Report results and exit with the appropriate code.
fn report(result: anyhow::Result<String>) {
    match result {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1)
        }
    }
}

fn limit(l: Limit) -> anyhow::Result<String> {
    let client = credit::client(&l.token)?;
    let rl = credit::rate_limit(&client)?;
    let json = serde_json::to_string(&rl)?;

    Ok(json)
}

fn repo(r: Repo) -> anyhow::Result<String> {
    let client = credit::client(&r.token)?;

    if r.repos.is_empty() {
        Err(anyhow!("No repositories given!"))
    } else {
        let (bads, goods): (Vec<_>, Vec<_>) = r
            .repos
            .par_iter()
            .map(|(owner, repo)| credit::repository_threads(&client, &owner, &repo))
            .partition_map(From::from);

        if !bads.is_empty() {
            eprintln!("There were some errors:");
            for e in bads {
                eprintln!("{}", e);
            }
        }

        if !goods.is_empty() {
            let zero = credit::Postings {
                issues: vec![],
                prs: vec![],
            };
            let all = goods.into_iter().fold(zero, |acc, ps| acc.combine(ps));
            let stats = all.statistics();

            if r.json {
                let json = serde_json::to_string(&stats)?;
                Ok(json)
            } else {
                let name = r.repos.iter().map(|(_, name)| name).join(", ");
                Ok(stats.report(&name))
            }
        } else {
            Err(anyhow!("No results to show!"))
        }
    }
}

fn split_repo(repo: &str) -> anyhow::Result<(String, String)> {
    let mut split = repo.split('/');
    let (owner, project) = split
        .next()
        .and_then(|owner| split.next().map(|project| (owner, project)))
        .ok_or_else(|| anyhow!("{}", repo))?;

    Ok((owner.to_string(), project.to_string()))
}
