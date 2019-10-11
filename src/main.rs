use std::{fs, path::{PathBuf, Path}};

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
}

fn main() -> Result<()> {
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
                let src = conf.src.as_ref().map(|p| &**p).unwrap_or_else(|| Path::new("<unknown>"));
                let dest = conf.dest.as_ref().map(|p| &**p).unwrap_or_else(|| Path::new("<unknown>"));
                println!("{} = Cloned from {}:{}, revision {}", dest.display(), url, src.display(), rev);
            }
        },
    }
    Ok(())
}
