//! A tool for measuring repository contributions.

mod github;

use anyhow::anyhow;
use gumdrop::{Options, ParsingStyle};
use itertools::Itertools;
use std::process;

/// A tool for measuring repository contributions.
#[derive(Debug, Options)]
struct Env {
    /// Print this help text
    help: bool,

    /// Github personal access token
    token: String,

    /// A Github repository to check (can pass multiple times)
    #[options(free, parse(try_from_str = "split_repo"))]
    repos: Vec<(String, String)>,

    /// Output as JSON
    json: bool,
}

fn main() {
    let env = Env::parse_args_or_exit(ParsingStyle::AllOptions);
    match work(env) {
        Ok(result) => println!("{}", result),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1)
        }
    }
}

fn work(env: Env) -> anyhow::Result<String> {
    let client = credit::client(&env.token)?;

    let issues = github::v4_issues(&client, "fosskers", "aura")?;

    println!("{:#?}", issues);

    Ok("Yay!".to_string())

    // if env.repos.is_empty() {
    //     Err(anyhow!("No repositories given!"))
    // } else {
    //     let (bads, goods): (Vec<_>, Vec<_>) = env
    //         .repos
    //         .iter()
    //         .map(|(owner, repo)| credit::repository_threads(&client, &owner, &repo))
    //         .partition_map(From::from);

    //     if !bads.is_empty() {
    //         eprintln!("There were some errors:");
    //         for e in bads {
    //             eprintln!("{}", e);
    //         }
    //     }

    //     if !goods.is_empty() {
    //         let zero = credit::Postings {
    //             issues: vec![],
    //             prs: vec![],
    //         };
    //         let all = goods.into_iter().fold(zero, |acc, ps| acc.combine(ps));
    //         let stats = all.statistics();

    //         if env.json {
    //             let json = serde_json::to_string(&stats)?;
    //             Ok(json)
    //         } else {
    //             let name = env.repos.iter().map(|(_, name)| name).join(", ");
    //             Ok(stats.report(&name))
    //         }
    //     } else {
    //         Err(anyhow!("No results to show!"))
    //     }
    // }
}

fn split_repo(repo: &str) -> anyhow::Result<(String, String)> {
    let mut split = repo.split('/');
    let (owner, project) = split
        .next()
        .and_then(|owner| split.next().map(|project| (owner, project)))
        .ok_or_else(|| anyhow!("{}", repo))?;

    Ok((owner.to_string(), project.to_string()))
}
