use gitlab_runner::{job::Job, JobHandler};
use tracing_subscriber::prelude::*;
use tracing::info;
use gitlab_runner::Runner;
use structopt::StructOpt;
use url::Url;


#[derive(StructOpt)]
struct Opts {
    #[structopt(env = "GITLAB_URL")]
    server: Url,
    #[structopt(env = "GITLAB_TOKEN")]
    token: String,
}

struct Run {
job: Job,
}

impl Run {
    pub fn new(job: Job) -> Self {
        Self { job }
    } 
}

#[async_trait::async_trait]
impl JobHandler for Run {
    async fn step(&mut self, script: &[String], _phase: gitlab_runner::Phase) -> gitlab_runner::JobResult {
        for command in script {
            info!("command is {:?}", command);
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
     let opts = Opts::from_args();
    let dir = tempfile::tempdir().unwrap();

    let (mut runner, layer) =
        Runner::new_with_layer(opts.server, opts.token, dir.path().to_path_buf());

    tracing_subscriber::Registry::default()
        .with(
            tracing_subscriber::fmt::Layer::new()
                .pretty()
                .with_filter(tracing::metadata::LevelFilter::INFO),
        )
        .with(layer)
        .init();
    runner.run(move |job| async move{
        Ok(Run::new(job))
    }, 8)
    .await
    .expect("Couldn't pick up jobs");

}
