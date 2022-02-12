mod interval;

use anyhow::{Error, Result};
use log::trace;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub(crate) use self::interval::SaveInterval;

pub(crate) async fn load_config() -> Result<Box<Config>> {
    trace!("loading config.yml to memory");
    let mut config_file = File::open("config.yml").await?;
    let config_file_len = config_file.metadata().await?.len();
    let mut config_file_bytes = Vec::with_capacity(config_file_len as usize);
    config_file.read_to_end(&mut config_file_bytes).await?;
    drop(config_file);
    trace!("parsing config.yml");
    let config_file: ConfigFile = serde_yaml::from_slice(&config_file_bytes)?;

    trace!("verifying config.yml");
    let preset = config_file.preset;
    let rcon_address = match config_file.rcon_address {
        Some(addr) => Some(addr.parse()?),
        None => match config_file.preset {
            None if config_file.commands_before.is_none()
                && config_file.commands_after.is_none() =>
            {
                None
            }
            Some(GamePreset::Minecraft) => Some("localhost:25575".parse().unwrap()),
            None => {
                return Err(Error::msg(
                    "rcon_address is required if no preset are defined",
                ))
            }
        },
    };
    let rcon_password = config_file.rcon_password;
    let commands_before = command_lines(
        config_file.commands_before.as_ref().map(String::as_str),
        config_file.preset,
        true,
    );
    let commands_after = command_lines(
        config_file.commands_after.as_ref().map(String::as_str),
        config_file.preset,
        false,
    );
    let backup_dir = config_file
        .backup_dir
        .or_else(|| std::env::var_os("BACKUP_DIR").map(PathBuf::from))
        .ok_or_else(|| Error::msg("backup_dir not found"))?;
    let save_dir = config_file
        .save_dir
        .or_else(|| std::env::var_os("SAVE_DIR").map(PathBuf::from))
        .ok_or_else(|| Error::msg("save_dir not found"))?;
    let backups = config_file
        .backups
        .into_iter()
        .map(|backup| BackupSetting {
            directory: backup_dir.join(&backup.name),
            name: backup.name,
            max_backups: backup.max_backups,
            interval: backup.interval,
            backup_mode: backup.backup_mode,
        })
        .collect();

    Ok(Box::new(Config {
        preset,
        rcon_address,
        rcon_password,
        commands_before,
        commands_after,
        save_dir,
        backups,
    }))
}

fn command_lines(str: Option<&str>, preset: Option<GamePreset>, before: bool) -> Vec<String> {
    match str {
        None => match preset {
            Some(preset) => preset.get_default_command(before),
            None => Vec::new(),
        },
        Some(s) if s.is_empty() => Vec::new(),
        Some(s) => s.lines().map(str::to_owned).collect(),
    }
}

#[derive(Debug)]
pub(crate) struct Config {
    /// the preset. this may be used to help rcon connection
    pub(crate) preset: Option<GamePreset>,
    /// the address to rcon server
    pub(crate) rcon_address: Option<SocketAddr>,
    /// the password of rcon
    pub(crate) rcon_password: String,
    /// the command will be ran before backup
    pub(crate) commands_before: Vec<String>,
    /// the command will be ran after backup
    pub(crate) commands_after: Vec<String>,
    /// the path to save directory
    pub(crate) save_dir: PathBuf,
    /// verified BackupSettings
    pub(crate) backups: Vec<BackupSetting>,
}

#[derive(Debug)]
pub(crate) struct BackupSetting {
    /// the name of backup setting
    pub(crate) name: String,
    /// the path to backup directory
    pub(crate) directory: PathBuf,
    /// the count of backups wil be kept
    pub(crate) max_backups: usize,
    /// the interval of backup.
    /// It's not allowed to be less than 5 minutes.
    pub(crate) interval: SaveInterval,
    /// the mode of backup
    pub(crate) backup_mode: BackupMode,
}

#[derive(Deserialize)]
struct ConfigFile {
    #[serde(default)]
    preset: Option<GamePreset>,
    #[serde(default)]
    rcon_address: Option<String>,
    #[serde(default)]
    rcon_password: String,
    #[serde(default)]
    commands_before: Option<String>,
    #[serde(default)]
    commands_after: Option<String>,
    backup_dir: Option<PathBuf>,
    save_dir: Option<PathBuf>,
    backups: Vec<BackupSettingFile>,
}

#[derive(Deserialize)]
struct BackupSettingFile {
    name: String,
    max_backups: usize,
    interval: SaveInterval,
    #[serde(default = "backup_mode_default")]
    backup_mode: BackupMode,
}

fn backup_mode_default() -> BackupMode {
    BackupMode::ModifiesOnly
}

#[derive(Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum GamePreset {
    Minecraft,
}

impl GamePreset {
    pub(super) fn get_default_command(&self, before: bool) -> Vec<String> {
        match self {
            GamePreset::Minecraft => {
                if before {
                    vec!["save-off".to_owned(), "save-all".to_owned()]
                } else {
                    vec!["save-on".to_owned()]
                }
            }
        }
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum BackupMode {
    Simple,
    /// this will replace previously newest backup with a backup only with modified files.
    ModifiesOnly,
    /// this will replace previously newest backup with a backup with bsdiff binary patch file.
    FileDiff,
}
