//! A tool for measuring repository contributions.

use anyhow::anyhow;
use gumdrop::{Options, ParsingStyle};
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

    /// A Github repository to check
    #[options(free, parse(try_from_str = "split_repo"))]
    repo: (String, String),

    /// Output as JSON
    json: bool,
}

fn main() {
    let env = Env::parse_args_or_exit(ParsingStyle::AllOptions);
    match work(&env) {
        Ok(result) => print!("{}", result),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1)
        }
    }
}

fn work(env: &Env) -> anyhow::Result<String> {
    let client = credit::client(&env.token)?;
    let postings = credit::repository_threads(&client, &env.repo.0, &env.repo.1)?;
    let stats = postings.statistics();

    if env.json {
        let json = serde_json::to_string(&stats)?;
        Ok(json)
    } else {
        Ok(format!("{:#?}", stats))
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
