use gitlab_runner::Runner;
use gitlab_runner::{job::Job, JobHandler};
use std::path::Path;
use structopt::StructOpt;
use tokio::process::Command;
use tracing::info;
use tracing_subscriber::prelude::*;
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
    async fn step(
        &mut self,
        script: &[String],
        _phase: gitlab_runner::Phase,
    ) -> gitlab_runner::JobResult {
        for command in script {
            info!("command is {:?}", command);
            let ci_project_url = self
                .job
                .variable("CI_PROJECT_URL")
                .expect("Can't find CI_PROJECT_URL in variables");
            let ci_commit_sha = self
                .job
                .variable("CI_COMMIT_SHA")
                .expect("Can't find CI_COMMIT_SHA in variables");
            let ci_project_title = self
                .job
                .variable("CI_PROJECT_TITLE")
                .expect("Can't find CI_PROJECT_TITLE in variables");
            info!("{}", ci_project_url.value());
            info!("{}", ci_commit_sha.value());

            let build_dir = self.job.build_dir();

            info!(
                "Clonning the repo in {} ...",
                build_dir.to_str().expect("can't fail")
            );
            let clone_output = Command::new("git")
                .current_dir(build_dir)
                .arg("clone")
                .arg(ci_project_url.value())
                .output()
                .await
                .expect("clonning the repo should not fail");
            assert!(clone_output.status.success());

            info!("Clonning the repo: DONE");

            info!(
                "project located at {}",
                build_dir
                    .join(Path::new(ci_project_title.value()))
                    .as_path()
                    .to_str()
                    .expect("can't fail")
            );
            let reset_head = Command::new("git")
                .current_dir(
                    build_dir
                        .join(Path::new(ci_project_title.value()))
                        .as_path(),
                )
                .arg("reset")
                .arg("--hard")
                .arg(ci_commit_sha.value())
                .output()
                .await
                .expect("git reset should not fail");
            assert!(reset_head.status.success());
            info!("Reset to commit {}: DONE", ci_commit_sha.value());

            let cargo_test = Command::new("cargo")
                .current_dir(
                    build_dir
                        .join(Path::new(ci_project_title.value()))
                        .as_path(),
                )
                .arg("test")
                .output()
                .await
                .expect("cargo test can't fail");
            assert!(cargo_test.status.success());
            info!(
                "cargo test succeed with {:#?}",
                String::from_utf8(cargo_test.stdout)
            );
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

    info!("temp dir path {:?}", dir);

    runner
        .run(move |job| async move { Ok(Run::new(job)) }, 8)
        .await
        .expect("Couldn't pick up jobs");
}
