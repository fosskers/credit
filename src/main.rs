//! A tool for measuring repository contributions.

use structopt::StructOpt;

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

#[derive(StructOpt)]
#[structopt(about = "A tool for measuring repository contributions")]
struct Env {
    /// Github personal access token
    #[structopt(long)]
    token: String,

    /// A Github repository to check
    #[structopt(name = "REPO")]
    repos: Vec<String>,

    /// Output as JSON
    #[structopt(short, long)]
    json: bool,
}

fn main() {
    let env = Env::from_args();

    println!("Hello, world!");
}
