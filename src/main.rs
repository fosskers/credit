//! A tool for measuring repository contributions.

use anyhow::anyhow;
use gumdrop::{Options, ParsingStyle};
use itertools::Itertools;
use std::process;

//- ~credit~: Just pull as much as possible via the Github API.
//- Who comments the most?
//- Average time to first issue response?
//- Average response time from Owner?
//- Who's PRs are getting merged?
//- Who is reviewing?
//- Query multiple repos at once and merge the results
//- Gotta call the endpoints I want manually.

//- Allow the supplying of start and end dates. This can be used to
//  track stats within a specific time period (say, some period in the past when
//  you worked on a specific project).

// Number of commits on `master` isn't counted - you can see that on Github :)

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

    if env.repos.is_empty() {
        Err(anyhow!("No repositories given!"))
    } else {
        let (bads, goods): (Vec<_>, Vec<_>) = env
            .repos
            .iter()
            .map(|(owner, repo)| credit::repository_threads(&client, &owner, &repo))
            .partition_map(|r| From::from(r));

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

            if env.json {
                let json = serde_json::to_string(&stats)?;
                Ok(json)
            } else {
                let name = env.repos.iter().map(|(_, name)| name).join(", ");
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
