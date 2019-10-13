use std::{
    env,
    ffi::OsString,
    iter,
    path::{PathBuf, Path},
    process::Command,
};

use anyhow::{ensure, Context, Result};
use git2::{IndexAddOption, RebaseOptions, Signature};
use git_subcopy::App;
use log::info;
use structopt::StructOpt;

#[derive(StructOpt)]
struct FetchOpts {
    /// The repository URL to extract files from
    url: String,
    /// The commit reference to extract files from
    rev: String,
    /// The source destination to extract files from
    upstream_path: PathBuf,
    /// The target destination to extract files from
    local_path: PathBuf,

    /// Whether or not to overwrite any existing directories. Will
    /// also create parent directories if they don't exist.
    #[structopt(short, long)]
    force: bool,
}

#[derive(StructOpt)]
enum Opt {
    /// Will fetch specific files from a git repository. This does
    /// nothing else other than copying those - it will not add this
    /// to your `.gitcopies` file.
    Fetch {
        #[structopt(flatten)]
        opts: FetchOpts,
    },
    /// Includes the operation for `fetch`, but will also add all
    /// relevant data to a `.gitcopies` file in the root of the
    /// repository.
    Add {
        #[structopt(flatten)]
        opts: FetchOpts,
    },
    /// List all subcopies according to the `.gitcopies` file.
    List,
    /// Get a shell in a temporary repository with a worktree clearly
    /// showing how your copy diverges from the upstream.
    Shell {
        /// The path to the copied content, as specified in
        /// `.gitcopies`.
        local_path: PathBuf,
    },
    /// Update changes on your local copy to be based on a newer
    /// upstream.
    Rebase {
        /// The path to the copied content, as specified in
        /// `.gitcopies`.
        local_path: PathBuf,
        /// The new revision to be based upon.
        rev: String,
    }
}

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::new()
            .default_filter_or("git_subcopy=info")
    );

    let opt = Opt::from_args();
    let app = App::new()?;

    match &opt {
        Opt::Fetch { opts }
        | Opt::Add { opts } => {
            let repo = app.fetch(&opts.url, true).context("failed to fetch git repo")?;

            ensure!(!opts.local_path.exists() || opts.force, "this could overwrite files, use --force if you're sure");

            let rev = repo.revparse_single(&opts.rev).context("failed to parse revision")?.id();
            app.extract(&repo, rev, &opts.upstream_path, &opts.local_path).context("failed to extract files")?;

            if let Opt::Add { .. } = &opt {
                app.register(&opts.url, rev, &opts.upstream_path, &opts.local_path).context("failed to register to .gitcopies")?;
            }
        },
        Opt::List => {
            let configs = app.list()?;

            for conf in configs.values() {
                let url = conf.url.as_ref().map(|p| &**p).unwrap_or("<unknown>");
                let rev = conf.rev.as_ref().map(|p| &**p).unwrap_or("<unknown>");
                let upstream_path = conf.upstream_path.as_ref().map(|p| &**p).unwrap_or_else(|| Path::new("<unknown>"));
                let local_path = &conf.local_path;
                println!("{} = Cloned from {}:{}, revision {}", local_path.display(), url, upstream_path.display(), rev);
            }
        },
        Opt::Shell { local_path } => {
            let conf = app.get(local_path)?;
            let shell = env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"));

            app.with_repo(&conf.url, &conf.rev, &conf.upstream_path, local_path, |repo| {
                println!("You are now in a shell inside of a temporary git repository.");
                println!("The upstream code is commited, and your changes in the worktree.");
                println!("When you exit this shell, your changed files will be copied back.");
                println!("=================================================================");
                Command::new(shell)
                    .current_dir(repo.workdir().expect("created repo shouldn't be a bare repo"))
                    .status()?;
                Ok(())
            })?;
        },
        Opt::Rebase { local_path, rev } => {
            let conf = app.get(local_path)?;
            let shell = env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"));

            let rev = app.with_repo(&conf.url, &conf.rev, &conf.upstream_path, local_path, |repo| {
                repo.find_remote("upstream").expect("remote 'upstream' should be set at this point")
                    .fetch(&[], None, None)?;

                let onto_rev = repo.revparse_single(&rev).context("failed to parse specified upstream revision")?;
                let onto_commit = repo.find_annotated_commit(onto_rev.id()).context("failed to find commit for revision")?;

                let head = repo.head().context("failed to find head")?
                    .peel_to_commit().context("head wasn't a commit")?;

                let tree_id = {
                    let mut index = repo.index().context("failed to open index")?;
                    index.add_all(iter::once("."), IndexAddOption::DEFAULT, None).context("failed to add to index")?;
                    index.write_tree().context("failed to write index to tree")?
                };
                let tree = repo.find_tree(tree_id).context("failed to find newly written tree")?;
                let sign = Signature::now("git-subcopy", "there's nobody to blame this time").context("failed to create signature")?;
                let id = repo.commit(Some("HEAD"), &sign, &sign, "Your changes", &tree, &[&head])
                    .context("failed to commit changes")?;

                let commit = repo.find_annotated_commit(id).context("failed to find new commit")?;

                info!("Rebasing...");
                repo.rebase(Some(&commit), None, Some(&onto_commit), Some(
                    RebaseOptions::new()
                        .quiet(false)
                        .inmemory(false)
                ))?;

                println!("A rebase is started. You're dropped into a shell to finish it.");
                println!("Run `git status` to see rebase progress, and");
                println!("`git rebase --continue` to continue the rebase.");
                println!("==============================================================");
                Command::new(shell)
                    .current_dir(repo.workdir().expect("created repo shouldn't be a bare repo"))
                    .status()?;
                Ok(onto_rev.id())
            })?;

            app.register(&conf.url, rev, &conf.upstream_path, &local_path).context("failed to register new rev")?;
        }
    }
    Ok(())
}
