//! A tool for measuring repository contributions.

use anyhow::anyhow;
use chrono::{DateTime, NaiveDate, Utc};
use gumdrop::{Options, ParsingStyle};
use indicatif::{MultiProgress, ProgressBar};
use itertools::Itertools;
use rayon::prelude::*;
use std::io::{self, Read};
use std::{process, thread};

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
    /// Find the most active users in a given area.
    Users(Users),
    /// Check the Github API for remaining rate limit allowance.
    Limit(Limit),
    /// Print markdown of JSON from a previous run of `credit repo --json`.
    Json(Json),
}

/// Analyse repository contributions.
#[derive(Options)]
struct Repo {
    /// Print this help text.
    help: bool,

    /// Github personal access token.
    #[options(required)]
    token: String,

    /// Look up Pull Request commit counts as well.
    commits: bool,

    /// Only consider contributions / comments after the given date.
    #[options(parse(try_from_str = "datetime"), meta = "YYYY-MM-DD")]
    start: Option<DateTime<Utc>>,

    /// Only consider contributions / comments before the given date.
    #[options(parse(try_from_str = "datetime"), meta = "YYYY-MM-DD")]
    end: Option<DateTime<Utc>>,

    /// Output as JSON.
    json: bool,

    /// Fetch Issues first, then PRs.
    serial: bool,

    /// A Github repository to check (can pass multiple times).
    #[options(free, parse(try_from_str = "split_repo"))]
    repos: Vec<(String, String)>,
}

/// Find the most active users in a given area.
#[derive(Options)]
struct Users {
    /// Print this help text.
    help: bool,

    /// Github personal access token.
    #[options(required)]
    token: String,

    /// The country to check.
    #[options(required)]
    location: String,
}

/// Check the Github API for remaining rate limit allowance.
#[derive(Options)]
struct Limit {
    /// Print this help text.
    help: bool,

    /// Github personal access token.
    #[options(required)]
    token: String,
}

/// Accept JSON from a previous run of `credit` through `stdin`, and print
/// the full Markdown output.
#[derive(Options)]
struct Json {
    /// Print this help text.
    help: bool,

    /// Show Pull Request commit counts.
    commits: bool,
}

fn main() {
    let args = Args::parse_args_or_exit(ParsingStyle::AllOptions);

    let result = match args.command {
        None => Err(anyhow!("No command specified. Did you mean to use `repo`?")),
        Some(Command::Limit(l)) => limit(l),
        Some(Command::Repo(r)) => repo(r),
        Some(Command::Users(u)) => users(u),
        Some(Command::Json(j)) => json(j),
    };

    report(result)
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

fn users(u: Users) -> anyhow::Result<String> {
    let client = credit::client(&u.token)?;
    let users = credit::user_contributions(&client, &u.location)?;
    Ok(users.to_string())
}

fn json(j: Json) -> anyhow::Result<String> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    let stats: credit::Statistics = serde_json::from_str(&buffer)?;

    Ok(stats.report("Unknown Project", j.commits))
}

fn limit(l: Limit) -> anyhow::Result<String> {
    let client = credit::client(&l.token)?;
    let rl = credit::rate_limit(&client)?;
    let json = serde_json::to_string(&rl)?;

    Ok(json)
}

fn repo(r: Repo) -> anyhow::Result<String> {
    let c = credit::client(&r.token)?;

    if r.repos.is_empty() {
        Err(anyhow!("No repositories given!"))
    } else {
        let m = MultiProgress::new();

        let spinners = r
            .repos
            .iter()
            .map(|(owner, repo)| {
                let issue_pb = m.add(ProgressBar::new_spinner());
                let pr_pb = m.add(ProgressBar::new_spinner());
                (issue_pb, pr_pb, owner, repo)
            })
            .collect::<Vec<_>>();

        // Apparently the thread itself doesn't need to be `join`ed for the
        // spinners to appear.
        thread::spawn(move || m.join_and_clear());

        let (bads, goods): (Vec<_>, Vec<_>) = spinners
            .par_iter()
            .map(|(ipb, ppb, owner, repo)| {
                credit::repo_threads(
                    &c, &ipb, &ppb, r.serial, r.commits, &r.start, &r.end, &owner, &repo,
                )
            })
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
                Ok(stats.report(&name, r.commits))
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

fn datetime(date: &str) -> anyhow::Result<DateTime<Utc>> {
    let naive = NaiveDate::parse_from_str(date, "%Y-%m-%d")?.and_hms(0, 0, 0);
    Ok(DateTime::from_utc(naive, Utc))
}
