pub mod error;
pub mod history;
pub mod local;
pub mod remote;

pub use history::Histories;
//pub use local::*;
//pub use remote::*;

use crate::borg;
use crate::prelude::*;

use gio::prelude::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path;
use zeroize::Zeroizing;

/// Compatibility config version
pub static VERSION: u16 = 1;

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Ord, Eq, PartialOrd, PartialEq)]
pub struct ConfigId(String);

impl ConfigId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl std::fmt::Display for ConfigId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Backup {
    #[serde(default)]
    pub config_version: u16,
    pub id: ConfigId,
    #[serde(default = "fake_repo_id")]
    pub repo_id: borg::RepoId,
    pub repo: Repository,
    pub encrypted: bool,
    #[serde(default)]
    pub encryption_mode: String,
    pub include: BTreeSet<path::PathBuf>,
    pub exclude: BTreeSet<Pattern>,
}

fn fake_repo_id() -> borg::RepoId {
    borg::RepoId::new(format!(
        "-randomid-{}",
        glib::uuid_string_random().to_string()
    ))
}

impl Backup {
    pub fn new(repo: Repository, info: borg::List, encrypted: bool) -> Self {
        let mut include = std::collections::BTreeSet::new();
        include.insert("".into());
        let mut exclude = std::collections::BTreeSet::new();
        exclude.insert(Pattern::PathPrefix(".cache".into()));

        Self {
            config_version: VERSION,
            id: ConfigId::new(glib::uuid_string_random().to_string()),
            repo,
            repo_id: info.repository.id,
            encrypted,
            encryption_mode: info.encryption.mode,
            include,
            exclude,
        }
    }

    pub fn include_dirs(&self) -> BTreeSet<path::PathBuf> {
        let mut dirs = BTreeSet::new();

        for dir in &self.include {
            dirs.insert(absolute(dir));
        }

        dirs
    }

    pub fn exclude_dirs_internal(&self) -> BTreeSet<Pattern> {
        let mut dirs = BTreeSet::new();

        for pattern in &self.exclude {
            match pattern {
                Pattern::PathPrefix(dir) => dirs.insert(Pattern::PathPrefix(absolute(dir))),
                other => dirs.insert(other.clone()),
            };
        }

        dirs.insert(Pattern::PathPrefix(absolute(path::Path::new(
            crate::REPO_MOUNT_DIR,
        ))));

        dirs
    }

    pub fn update_version_0(&mut self, info: borg::List, icon_symbolic_new: Option<gio::Icon>) {
        if self.config_version == 0 {
            self.config_version = 1;

            if let Repository::Local(local::Repository {
                ref mut icon_symbolic,
                ..
            }) = self.repo
            {
                *icon_symbolic = icon_symbolic_new
                    .and_then(|icon| gio::IconExt::to_string(&icon))
                    .as_ref()
                    .map(ToString::to_string);
            }
            self.repo_id = info.repository.id;
            self.encryption_mode = info.encryption.mode;
        }
    }
}

impl LookupConfigId<Backup> for Backups {
    fn get_mut_result(&mut self, key: &ConfigId) -> Result<&mut Backup, error::BackupNotFound> {
        self.iter_mut()
            .find(|x| x.id == *key)
            .ok_or_else(|| error::BackupNotFound::new(key.clone()))
    }

    fn get_result(&self, key: &ConfigId) -> Result<&Backup, error::BackupNotFound> {
        self.iter()
            .find(|x| x.id == *key)
            .ok_or_else(|| error::BackupNotFound::new(key.clone()))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Pattern {
    PathPrefix(path::PathBuf),
    #[serde(
        deserialize_with = "deserialize_regex",
        serialize_with = "serialize_regex"
    )]
    RegularExpression(Box<regex::Regex>),
}

fn deserialize_regex<'de, D>(deserializer: D) -> Result<Box<regex::Regex>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;
    regex::Regex::new(&string)
        .map(Box::new)
        .map_err(serde::de::Error::custom)
}

fn serialize_regex<S>(regex: &regex::Regex, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(regex.as_str())
}

impl std::cmp::PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.pattern() == other.pattern()
    }
}
impl std::cmp::Eq for Pattern {}

impl std::cmp::Ord for Pattern {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.pattern().cmp(&other.pattern())
    }
}

impl std::cmp::PartialOrd for Pattern {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Pattern {
    pub fn is_match(&self, path: &std::path::Path) -> bool {
        match self {
            Self::PathPrefix(path_prefix) => path.starts_with(path_prefix),
            Self::RegularExpression(regex) => regex.is_match(&path.to_string_lossy()),
        }
    }
    pub fn selector(&self) -> String {
        match self {
            Self::PathPrefix(_) => "pp",
            Self::RegularExpression(_) => "re",
        }
        .to_string()
    }

    pub fn pattern(&self) -> String {
        match self {
            Self::PathPrefix(path) => path.to_string_lossy().to_string(),
            Self::RegularExpression(pattern) => pattern.as_str().to_string(),
        }
    }
}

pub type Password = Zeroizing<Vec<u8>>;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Repository {
    Local(local::Repository),
    Remote(remote::Repository),
}

impl Repository {
    pub fn icon(&self) -> String {
        match self {
            Self::Local(local) => local.icon.clone().unwrap_or_else(|| String::from("folder")),
            Self::Remote(_) => String::from("network-server"),
        }
    }

    pub fn icon_symbolic(&self) -> String {
        match self {
            Self::Local(local) => local
                .icon_symbolic
                .clone()
                .unwrap_or_else(|| String::from("folder-symbolic")),
            Self::Remote(_) => String::from("network-server-symbolic"),
        }
    }

    pub fn location(&self) -> String {
        if let Self::Local(local) = self {
            format!(
                "{} – {}",
                local.mount_name.as_deref().unwrap_or_default(),
                self.get_subtitle(),
            )
        } else {
            self.to_string()
        }
    }

    pub fn get_uri_fuse(&self) -> Option<String> {
        match self {
            Self::Local(local::Repository { uri: Some(uri), .. })
                if !gio::File::new_for_uri(&uri).is_native() =>
            {
                Some(uri.clone())
            }
            _ => None,
        }
    }

    pub fn get_subtitle(&self) -> String {
        match self {
            Self::Local(local) => local
                .drive_name
                .clone()
                .or_else(|| self.get_uri_fuse())
                .unwrap_or_else(|| self.to_string()),
            Self::Remote(_) => self.to_string(),
        }
    }

    pub fn set_settings(&mut self, settings: Option<BackupSettings>) {
        *match self {
            Self::Local(local) => &mut local.settings,
            Self::Remote(remote) => &mut remote.settings,
        } = settings;
    }

    pub fn get_settings(&self) -> Option<BackupSettings> {
        match self {
            Self::Local(local) => &local.settings,
            Self::Remote(remote) => &remote.settings,
        }
        .clone()
    }
}

impl std::fmt::Display for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let repo = match self {
            Self::Local(local) => local.path().to_string_lossy().to_string(),
            Self::Remote(remote) => remote.uri.to_string(),
        };
        write!(f, "{}", repo)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackupSettings {
    pub command_line_args: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Backups(Vec<Backup>);

impl Backups {
    pub fn from_default_path() -> std::io::Result<Self> {
        Self::from_path(&Self::default_path()?)
    }

    pub fn from_path(path: &std::path::Path) -> std::io::Result<Self> {
        #[derive(Serialize, Deserialize, Debug)]
        struct BackupsLegacy {
            backups: BTreeMap<ConfigId, Backup>,
        }

        let conf: std::result::Result<Self, _> =
            serde_json::de::from_reader(std::fs::File::open(&path)?);

        // pre v2 parser
        match conf {
            Ok(conf) => Ok(conf),
            Err(err) => {
                let conf_legacy: std::result::Result<BackupsLegacy, _> =
                    serde_json::de::from_reader(std::fs::File::open(path)?);
                match conf_legacy {
                    Ok(legacy) => Ok(Self(legacy.backups.into_iter().map(|x| x.1).collect())),
                    Err(_) => Err(err.into()),
                }
            }
        }
    }

    pub fn exists(&self, id: &ConfigId) -> bool {
        self.iter().any(|x| x.id == *id)
    }

    pub fn insert(&mut self, new: Backup) -> Result<(), error::BackupExists> {
        if self.exists(&new.id) {
            Err(error::BackupExists { id: new.id })
        } else {
            self.0.push(new);
            Ok(())
        }
    }

    pub fn remove(&mut self, remove: &ConfigId) -> Result<(), error::BackupNotFound> {
        if !self.exists(remove) {
            Err(error::BackupNotFound::new(remove.clone()))
        } else {
            self.0.retain(|x| x.id != *remove);
            Ok(())
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Backup> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, Backup> {
        self.0.iter_mut()
    }

    pub fn default_path() -> std::io::Result<std::path::PathBuf> {
        crate::utils::prepare_config_file("backup.json", Self::default())
    }
}

pub fn absolute(path: &path::Path) -> path::PathBuf {
    HOME_DIR.join(path)
}
