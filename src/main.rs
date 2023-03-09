//! A tool for measuring repository contributions.

use anyhow::{anyhow, Context};
use chrono::{DateTime, NaiveDate, Utc};
use gumdrop::{Options, ParsingStyle};
use indicatif::{MultiProgress, ProgressBar};
use itertools::Itertools;
use rayon::prelude::*;
use serde::Deserialize;
use std::io::{self, Read};
use std::{process, thread};

/// Config that can be set in a `credit.toml` file.
#[derive(Deserialize, Default)]
struct Config {
    token: Option<String>,
}

/// A tool for measuring repository contributions.
#[derive(Options)]
struct Args {
    /// Print this help text.
    help: bool,

    /// Print the current version of credit.
    version: bool,

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

impl Command {
    fn token(&self) -> Option<String> {
        match self {
            Command::Repo(r) => r.token.clone(),
            Command::Users(u) => u.token.clone(),
            Command::Limit(l) => l.token.clone(),
            Command::Json(_) => None,
        }
    }
}

/// Analyse repository contributions.
#[derive(Options)]
struct Repo {
    /// Print this help text.
    help: bool,
    /// Github personal access token.
    token: Option<String>,
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
    #[options(default = "10")]
    limit: usize,
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
    token: Option<String>,
    /// The country to check.
    #[options(required)]
    location: String,
    /// Output as JSON.
    json: bool,
}

/// Check the Github API for remaining rate limit allowance.
#[derive(Options)]
struct Limit {
    /// Print this help text.
    help: bool,
    /// Github personal access token.
    token: Option<String>,
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

    match args.command {
        _ if args.version => {
            let version = env!("CARGO_PKG_VERSION");
            println!("{}", version);
        }
        None => {
            eprintln!("{}", Args::usage());
            std::process::exit(1);
        }
        Some(cmd) => report(work(cmd)),
    }
}

fn work(command: Command) -> anyhow::Result<String> {
    let mut config_path = xdg::BaseDirectories::new()?.get_config_home();
    config_path.push("credit.toml");
    let config: Config = std::fs::read_to_string(config_path)
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default();

    match command {
        Command::Limit(_) => match command.token().or(config.token) {
            None => Err(anyhow!("No token given!")),
            Some(token) => limit(&token),
        },
        Command::Repo(ref r) => match command.token().or(config.token) {
            None => Err(anyhow!("No token given!")),
            Some(token) => repo(&token, r),
        },
        Command::Users(ref u) => match command.token().or(config.token) {
            None => Err(anyhow!("No token given!")),
            Some(token) => users(&token, u),
        },
        Command::Json(j) => json(j),
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

fn users(token: &str, u: &Users) -> anyhow::Result<String> {
    let users = credit::user_contributions(token, &u.location)?;

    if u.json {
        let json = serde_json::to_string(&users)?;
        Ok(json)
    } else {
        Ok(users.to_string())
    }
}

fn json(j: Json) -> anyhow::Result<String> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    let stats: credit::Statistics = serde_json::from_str(&buffer)?;

    Ok(stats.report("Unknown Project", 10, j.commits))
}

fn limit(token: &str) -> anyhow::Result<String> {
    let rl = credit::rate_limit(token)?;
    let json = serde_json::to_string(&rl)?;

    Ok(json)
}

fn repo(token: &str, r: &Repo) -> anyhow::Result<String> {
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
                    token, &ipb, &ppb, r.serial, r.commits, &r.start, &r.end, &owner, &repo,
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
                Ok(stats.report(&name, r.limit, r.commits))
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
    let naive = NaiveDate::parse_from_str(date, "%Y-%m-%d")?
        .and_hms_opt(0, 0, 0)
        .context("Failed to parse date.")?;

    Ok(DateTime::from_utc(naive, Utc))
}
