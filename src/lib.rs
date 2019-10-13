use std::{collections::HashMap, fs, path::{PathBuf, Path}};

use anyhow::{anyhow, Context, Result};
use git2::{
    build::RepoBuilder,
    Config,
    Oid,
    Repository,
    ResetType,
    TreeWalkMode,
    TreeWalkResult,
};
use log::debug;
use tempfile::Builder;
use walkdir::WalkDir;

fn path_to_string(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| anyhow!("path must be valid utf-8"))
}

#[derive(Debug, Default)]
pub struct SubcopyConfigOption {
    pub url: Option<String>,
    pub rev: Option<String>,
    pub upstream_path: Option<PathBuf>,
    pub local_path: PathBuf,
}
#[derive(Debug, Default)]
pub struct SubcopyConfig {
    pub url: String,
    pub rev: String,
    pub upstream_path: PathBuf,
}

pub struct App {
    cache_dir: PathBuf,
}
impl App {
    pub fn new() -> Result<Self> {
        Ok(Self {
            cache_dir: dirs::cache_dir().map(|mut path| {
                path.push(env!("CARGO_PKG_NAME"));
                path
            }).ok_or_else(|| anyhow!("can't choose a cache directory"))?,
        })
    }

    pub fn fetch(&self, url: &str) -> Result<Repository> {
        let path = self.cache_dir.join(base64::encode_config(url, base64::URL_SAFE_NO_PAD));

        if path.exists() {
            let repo = Repository::open_bare(&path).context("failed to open cached bare repository")?;
            repo.remote_anonymous(url).context("failed to create anonymous remote")?
                .fetch(&[], None, None).context("failed to fetch from anonymous remote")?;
            Ok(repo)
        } else {
            Ok(RepoBuilder::new()
               .bare(true)
               .clone(url, &path)
               .context("failed to clone repository")?)
        }
    }

    pub fn extract(&self, repo: &'_ Repository, rev: Oid, upstream_path: &Path, local_path: &Path) -> Result<()> {
        let tree = repo.find_tree(rev).context("failed to find tree at revision")?;
        let entry = tree.get_path(upstream_path).context("failed to get path")?;
        let object = entry.to_object(&repo).context("failed to get path's object")?;

        if let Ok(blob) = object.peel_to_blob() {
            let path = local_path.join(entry.name().ok_or_else(|| anyhow!("name is not utf-8 encoded"))?);
            fs::write(path, blob.content())?;
        } else {
            let tree = object.peel_to_tree()?;

            let mut error = None;
            tree.walk(TreeWalkMode::PreOrder, |dir, entry| {
                let inner = || -> Result<()> {
                    let object = entry.to_object(&repo)?;
                    let mut path = local_path.join(dir);
                    path.push(entry.name().ok_or_else(|| anyhow!("name is not utf-8 encoded"))?);

                    if let Ok(blob) = object.peel_to_blob() {
                        fs::write(path, blob.content())?;
                    } else if object.peel_to_tree().is_ok() {
                        fs::create_dir_all(path)?;
                    }
                    Ok(())
                };
                match inner() {
                    Ok(()) => TreeWalkResult::Ok,
                    Err(err) => {
                        error = Some(err);
                        TreeWalkResult::Abort
                    }
                }
            })?;
            if let Some(err) = error {
                return Err(err);
            }
        }
        Ok(())
    }

    pub fn canonicalize(&self, repo: &Repository, local_path: &Path) -> Result<PathBuf> {
        let workdir = repo.workdir().ok_or_else(|| anyhow!("repository is bare and has no workdir"))?
            .canonicalize().context("failed to find full path to repository workdir")?;
        let local_path = local_path.canonicalize().context("failed to find full path to destination directory")?;
        let relative = local_path.strip_prefix(&workdir).context("destination directory not in a repository")?;

        Ok(relative.to_path_buf())
    }

    pub fn register(&self, url: &str, rev: Oid, upstream_path: &Path, local_path: &Path) -> Result<()> {
        let repo = Repository::open_from_env()?;
        let relative = self.canonicalize(&repo, local_path)?;
        let workdir = repo.workdir().expect("canonicalize has already checked this");

        let relative_str = path_to_string(&relative)?;

        let mut config = Config::open(&workdir.join(".gitcopies")).context("failed to open .gitcopies")?;
        config.set_str(&format!("subcopy.{}.url", relative_str), url)?;
        config.set_str(&format!("subcopy.{}.rev", relative_str), &rev.to_string())?;
        config.set_str(&format!("subcopy.{}.upstreamPath", relative_str), path_to_string(upstream_path)?)?;
        Ok(())
    }

    pub fn list(&self) -> Result<HashMap<String, SubcopyConfigOption>> {
        let repo = Repository::open_from_env()?;
        let workdir = repo.workdir().ok_or_else(|| anyhow!("repository is bare and has no workdir"))?;
        let mut config = Config::open(&workdir.join(".gitcopies")).context("failed to open .gitcopies")?;
        let snapshot = config.snapshot().context("failed to take a snapshot of config")?;

        let mut map: HashMap<String, SubcopyConfigOption> = HashMap::new();

        for entry in &snapshot.entries(Some(r"^subcopy\..*\.(url|rev|upstreamPath)$")).context("failed to iter config entries")? {
            let entry = entry.context("failed to read config entry")?;
            let name = entry.name().ok_or_else(|| anyhow!("entry name was not valid utf-8"))?;

            let withoutend = name.rsplitn(2, '.').nth(1).ok_or_else(|| anyhow!("incomplete subcopy property name"))?;
            let middle = withoutend.splitn(2, '.').nth(1).ok_or_else(|| anyhow!("incomplete subcopy property name"))?;
            let slot = map.entry(middle.to_owned()).or_insert_with(|| SubcopyConfigOption {
                local_path: PathBuf::from(&middle),
                ..SubcopyConfigOption::default()
            });

            if name.ends_with("url") {
                slot.url = entry.value().map(String::from);
            } else if name.ends_with("rev") {
                slot.rev = entry.value().map(String::from);
            } else if name.ends_with("upstreamPath") {
                slot.upstream_path = entry.value().map(PathBuf::from);
            }
        }

        Ok(map)
    }

    pub fn get(&self, key: &Path) -> Result<SubcopyConfig> {
        let repo = Repository::open_from_env()?;
        let key = self.canonicalize(&repo, key)?;

        let workdir = repo.workdir().ok_or_else(|| anyhow!("repository is bare and has no workdir"))?;
        let mut config = Config::open(&workdir.join(".gitcopies")).context("failed to open .gitcopies")?;
        let snapshot = config.snapshot().context("failed to take a snapshot of config")?;

        let key = path_to_string(&key)?;

        Ok(SubcopyConfig {
            url: snapshot.get_string(&format!("subcopy.{}.url", key))?,
            rev: snapshot.get_string(&format!("subcopy.{}.rev", key))?,
            upstream_path: snapshot.get_path(&format!("subcopy.{}.upstreamPath", key))?,
        })
    }

    pub fn with_repo<F, T>(&self, url: &str, rev: &str, upstream_path: &Path, local_path: &Path, callback: F) -> Result<T>
    where
        F: FnOnce(&Repository) -> Result<T>,
    {
        let tmp = Builder::new().prefix("git-subcopy").tempdir().context("failed to get temporary directory")?;
        let upstream_repo = {
            let upstream_bare = self.fetch(url).context("failed to fetch source repository")?;
            let upstream_bare_path = upstream_bare.path().canonicalize().context("failed to get full cache path")?;
            let upstream_str = format!("file://{}", path_to_string(&upstream_bare_path)?);

            Repository::clone(&upstream_str, tmp.path())
                .context("failed to clone cache of upstream repository")?
        };

        upstream_repo.remote("upstream", url).context("failed to add upstream remote")?;

        let rev = upstream_repo.revparse_single(rev).context("failed to parse revision")?;
        upstream_repo.reset(&rev, ResetType::Hard, None).context("failed to reset repository")?;

        let upstream_path = tmp.path().join(upstream_path);

        for entry in WalkDir::new(local_path) {
            let entry = entry.context("failed to read directory entry")?;

            let from = entry.path();
            let to_relative = entry.path().strip_prefix(local_path).context("walkdir should always have prefix")?;
            let to = upstream_path.join(to_relative);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&to).context("failed to copy dir")?;
            } else {
                debug!("{} -> {}", from.display(), to.display());
                fs::copy(from, &to).context("failed to copy file")?;
            }
        }

        let ret = callback(&upstream_repo)?;

        for entry in WalkDir::new(&upstream_path).into_iter().filter_entry(|e| e.file_name().to_str() != Some(".git")) {
            let entry = entry.context("failed to read directory entry")?;

            let from = entry.path();
            let to_relative = entry.path().strip_prefix(&upstream_path).context("walkdir should always have prefix")?;
            let to = local_path.join(to_relative);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&to).context("failed to copy dir")?;
            } else {
                debug!("{} -> {}", from.display(), to.display());
                fs::copy(from, &to).context("failed to copy file")?;
            }
        }

        Ok(ret)
    }
}
