use std::{collections::HashMap, fs, path::{PathBuf, Path}};

use anyhow::{anyhow, Context, Result};
use git2::{
    build::RepoBuilder,
    Config,
    Repository,
    TreeWalkMode,
    TreeWalkResult,
};

fn path_to_string(path: &Path) -> Result<&str> {
    path.to_str().ok_or_else(|| anyhow!("path must be valid utf-8"))
}

#[derive(Debug, Default)]
pub struct SubcopyConfig {
    pub url: Option<String>,
    pub rev: Option<String>,
    pub src: Option<PathBuf>,
    pub dest: Option<PathBuf>,
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

    pub fn extract(&self, repo: &'_ Repository, rev: &str, src: &Path, dest: &Path) -> Result<()> {
        let tree = repo.revparse_single(rev).context("failed to parse revision")?
            .peel_to_tree().context("revision was not tree-like")?;
        let entry = tree.get_path(src).context("failed to get path")?;
        let object = entry.to_object(&repo).context("failed to get path's object")?;

        if let Ok(blob) = object.peel_to_blob() {
            let path = dest.join(entry.name().ok_or_else(|| anyhow!("name is not utf-8 encoded"))?);
            fs::write(path, blob.content())?;
        } else {
            let tree = object.peel_to_tree()?;

            let mut error = None;
            tree.walk(TreeWalkMode::PreOrder, |dir, entry| {
                let inner = || -> Result<()> {
                    let object = entry.to_object(&repo)?;
                    let mut path = dest.join(dir);
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

    pub fn register(&self, url: &str, rev: &str, src: &Path, dest: &Path) -> Result<()> {
        let repo = Repository::open_from_env()?;
        let workdir = repo.workdir().ok_or_else(|| anyhow!("repository is bare and has no workdir"))?
            .canonicalize().context("failed to find full path to repository workdir")?;
        let dest = dest.canonicalize().context("failed to find full path to destination directory")?;
        let relative = dest.strip_prefix(&workdir).context("destination directory not in a repository")?;

        let relative_str = path_to_string(relative)?;

        let mut config = Config::open(&workdir.join(".gitcopies")).context("failed to open .gitcopies")?;
        config.set_str(&format!("subcopy.{}.url", relative_str), url)?;
        config.set_str(&format!("subcopy.{}.rev", relative_str), rev)?;
        config.set_str(&format!("subcopy.{}.src", relative_str), path_to_string(src)?)?;
        config.set_str(&format!("subcopy.{}.dest", relative_str), path_to_string(relative)?)?;
        Ok(())
    }

    pub fn list(&self) -> Result<HashMap<String, SubcopyConfig>> {
        let repo = Repository::open_from_env()?;
        let workdir = repo.workdir().ok_or_else(|| anyhow!("repository is bare and has no workdir"))?;
        let config = Config::open(&workdir.join(".gitcopies")).context("failed to open .gitcopies")?;

        let mut map: HashMap<String, SubcopyConfig> = HashMap::new();

        for entry in &config.entries(Some(r"^subcopy\..*\.(url|rev|src|dest)$"))? {
            let entry = entry?;
            let name = entry.name().ok_or_else(|| anyhow!("entry name was not valid utf-8"))?;

            let part = name.rsplitn(2, '.').nth(1).ok_or_else(|| anyhow!("incomplete subcopy property name"))?;
            let slot = map.entry(part.to_owned()).or_default();

            if name.ends_with("url") {
                slot.url = entry.value().map(String::from);
            } else if name.ends_with("rev") {
                slot.rev = entry.value().map(String::from);
            } else if name.ends_with("src") {
                slot.src = entry.value().map(PathBuf::from);
            } else if name.ends_with("dest") {
                slot.dest = entry.value().map(PathBuf::from);
            }
        }

        Ok(map)
    }
}
