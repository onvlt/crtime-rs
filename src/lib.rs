use chrono::{DateTime, Utc};
use std::error::Error;
use std::fs;
use std::fs::DirEntry;
use std::io;
use std::io::BufRead;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Config<'a> {
    pub dir: &'a Path,
}

impl<'a> Config<'a> {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("Not enough arguments");
        }

        let dir = &args[1];

        Ok(Config {
            dir: &Path::new(dir),
        })
    }
}

#[derive(Debug)]
struct FsItem {
    created: DateTime<Utc>,
    name: String,
    new_name: String,
    path: PathBuf,
    new_path: PathBuf,
}

#[derive(Debug)]
enum FsItemError {
    Io(io::Error),
    ItemIsDir,
    NameFailed,
    ParentFailed,
}

#[derive(Debug)]
struct FsItemRenameError<'a> {
    item: &'a FsItem,
    reason: io::Error,
}

impl std::convert::From<io::Error> for FsItemError {
    fn from(error: io::Error) -> Self {
        FsItemError::Io(error)
    }
}

type ItemResult = Result<FsItem, FsItemError>;

impl FsItem {
    pub fn new(entry: io::Result<DirEntry>) -> ItemResult {
        let entry = entry?;
        let path = entry.path().to_path_buf();
        let meta = entry.metadata()?;
        let created = meta.created()?;
        let created = DateTime::<Utc>::from(created);

        if meta.is_dir() {
            return Err(FsItemError::ItemIsDir);
        }

        let name = match path.iter().last() {
            Some(last) => match last.to_str() {
                Some(name) => name,
                None => return Err(FsItemError::NameFailed),
            },
            None => return Err(FsItemError::NameFailed),
        };

        let new_name = format!("{} {}", created.format("%Y%m%d%M%S"), name);

        let new_path = match path.parent() {
            Some(parent) => parent.join(&new_name),
            None => return Err(FsItemError::ParentFailed),
        };

        Ok(FsItem {
            created,
            name: name.to_owned(),
            new_name,
            path,
            new_path,
        })
    }

    pub fn rename(&self) -> Result<&Self, FsItemRenameError> {
        match fs::rename(&self.path, &self.new_path) {
            Ok(()) => Ok(self),
            Err(error) => Err(FsItemRenameError {
                item: self,
                reason: error,
            })
        }
    }
}

fn partition_results<I, T, E>(iter: I) -> (impl Iterator<Item = T>, impl Iterator<Item = E>) where
    I: Iterator<Item=Result<T, E>>,
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    let (oks, errs): (Vec<_>, Vec<_>) = iter.partition(Result::is_ok);

    let oks = oks.into_iter().map(Result::unwrap);
    let errs = errs.into_iter().map(Result::unwrap_err);

    (oks, errs)
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    println!("Directory: {}", config.dir.display());

    let dir = config.dir.read_dir()?;
    let mut items: Vec<_> = dir.map(FsItem::new).filter_map(Result::ok).collect();

    items.sort_by(|a, b| a.created.partial_cmp(&b.created).unwrap());

    for item in &items {
        println!("Rename: {} -> {}", item.name, item.new_name);
    }

    let stdin = io::stdin();

    match stdin.lock().lines().next() {
        Some(Ok(ref line)) if line == "Y" => {
            let items = items.iter().map(|item| item.rename());

            let (oks, errs) = partition_results(items);

            let oks: Vec<_> = oks.collect();
            let errs: Vec<_> = errs.collect();

            println!("\nRenamed items:");

            for item in oks {
                println!("- {} -> {}", item.name, item.new_name);
            }

            println!("\nFailed:");

            for err in errs {
                println!("- {} -> {}: {}",
                    err.item.name,
                    err.item.new_name,
                    err.reason
                );
            }

            println!("Ok");
        }
        _ => println!("Renaming cancelled."),
    }

    Ok(())
}
