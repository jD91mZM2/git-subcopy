use std::{
    env,
    ffi::OsString,
    fs,
    path::{PathBuf, Path},
    process::Command,
};

use anyhow::{Context, Result};
use git_subcopy::App;
use structopt::StructOpt;

#[derive(StructOpt)]
struct FetchOpts {
    /// The repository URL to extract files from
    url: String,
    /// The commit reference to extract files from
    rev: String,
    /// The source destination to extract files from
    from: PathBuf,
    /// The target destination to extract files from
    to: PathBuf,

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
    /// Get a shell in a temporary repository with a worktree clearly showing how your copy diverges from the upstream.
    Shell {
        /// The path to the copied content, as specified in
        /// `.gitcopies`.
        from: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    let opt = Opt::from_args();
    let app = App::new()?;

    match &opt {
        Opt::Fetch { opts }
        | Opt::Add { opts } => {
            let repo = app.fetch(&opts.url).context("failed to fetch git repo")?;

            if opts.force {
                fs::create_dir_all(&opts.to).context("failed to create destination directory")?;
            } else {
                fs::create_dir(&opts.to).context("failed to create *unique* destination directory")?;
            }

            app.extract(&repo, &opts.rev, &opts.from, &opts.to).context("failed to extract files")?;

            if let Opt::Add { .. } = &opt {
                app.register(&opts.url, &opts.rev, &opts.from, &opts.to).context("failed to register to .gitcopies")?;
            }
        },
        Opt::List => {
            let configs = app.list()?;

            for conf in configs.values() {
                let url = conf.url.as_ref().map(|p| &**p).unwrap_or("<unknown>");
                let rev = conf.rev.as_ref().map(|p| &**p).unwrap_or("<unknown>");
                let source_path = conf.source_path.as_ref().map(|p| &**p).unwrap_or_else(|| Path::new("<unknown>"));
                let dest_path = &conf.dest_path;
                println!("{} = Cloned from {}:{}, revision {}", dest_path.display(), url, source_path.display(), rev);
            }
        },
        Opt::Shell { from } => {
            let conf = app.get(from)?;

            let shell = env::var_os("SHELL").unwrap_or_else(|| OsString::from("/bin/sh"));

            app.with_repo(&conf.url, &conf.rev, &conf.source_path, from, |repo| {
                Command::new(shell)
                    .current_dir(repo.workdir().expect("created repo shouldn't be a bare repo"))
                    .status()?;
                Ok(())
            })?;
        },
    }
    Ok(())
}
