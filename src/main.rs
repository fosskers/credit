//! A tool for measuring repository contributions.

use gumdrop::{Options, ParsingStyle};

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
    #[options(free)]
    repos: Vec<String>,

    /// Output as JSON
    json: bool,
}

fn main() {
    let env = Env::parse_args_or_exit(ParsingStyle::AllOptions);
    match work(&env) {
        Err(e) => eprintln!("Crap: {:?}", e),
        Ok(_) => (),
    }
}

fn work(env: &Env) -> anyhow::Result<()> {
    println!("{:#?}", env);

    let client = credit::client(&env.token)?;
    let postings = credit::repository_threads(&client, "kadena-io", "chainweb-node")?;
    let stats = postings.statistics();

    println!("{:#?}", stats);

    Ok(())
}
