mod config;
mod tar;

use self::tar::append_dir_all_sorted;
use crate::config::{load_config, BackupMode, BackupSetting, Config, GamePreset};
use anyhow::Result;
use anyhow::{Context as _, Error};
use chrono::{Duration, NaiveDateTime, NaiveTime, Timelike, Utc};
use futures::future::{join_all, try_join_all};
use log::{error, trace};
use std::fs::File as StdFile;
use std::io::{BufWriter, ErrorKind, SeekFrom, Write};
use std::path::Path;
use tokio::fs::{remove_file, rename, File, OpenOptions};
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::task::spawn_blocking;

type Connection = rcon::Connection<tokio::net::TcpStream>;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let config = load_config()
        .await
        .with_context(|| "loading config file (config.yml)")?;

    trace!("load config: {:?}", config);

    let mut ctx = Context::new(&config);
    main_loop(&mut ctx).await;

    Ok(())
}

async fn main_loop(ctx: &mut Context<'_>) -> ! {
    let mut begin = chrono::Utc::now().naive_utc();

    loop {
        tokio::time::sleep(compute_sleep_time(begin.time())).await;
        let end = chrono::Utc::now().naive_utc();
        let dur = end.signed_duration_since(begin);

        trace!("finish sleep. it's {} now. {} passed.", end, dur);

        if Duration::zero() < dur {
            if let Some(err) = do_step(ctx, &begin, &end).await.err() {
                error!("error during backup step at {}: {}", end, err)
            }
        }

        begin = end
    }
}

fn compute_sleep_time(now: NaiveTime) -> std::time::Duration {
    let until = match now.minute() {
        00..=04 => NaiveTime::from_hms(now.hour(), 05, 0),
        05..=09 => NaiveTime::from_hms(now.hour(), 10, 0),
        10..=14 => NaiveTime::from_hms(now.hour(), 15, 0),
        15..=19 => NaiveTime::from_hms(now.hour(), 20, 0),
        20..=24 => NaiveTime::from_hms(now.hour(), 25, 0),
        25..=29 => NaiveTime::from_hms(now.hour(), 30, 0),
        30..=34 => NaiveTime::from_hms(now.hour(), 35, 0),
        35..=39 => NaiveTime::from_hms(now.hour(), 40, 0),
        40..=44 => NaiveTime::from_hms(now.hour(), 45, 0),
        45..=49 => NaiveTime::from_hms(now.hour(), 50, 0),
        50..=54 => NaiveTime::from_hms(now.hour(), 55, 0),
        55..=59 => {
            if now.hour() == 23 {
                NaiveTime::from_hms_nano(23, 59, 59, 1_000_000_000)
            } else {
                NaiveTime::from_hms(now.hour() + 1, 00, 0)
            }
        }
        _ => unreachable!(),
    };

    let duration = (until - Utc::now().time()).to_std().unwrap();

    trace!("wait for {:?} to reach {}", duration, until);

    duration
}

#[test]
fn compute_sleep_time_test() {
    use std::time::Duration as StdDuration;

    assert_eq!(
        compute_sleep_time(NaiveTime::from_hms(0, 0, 0)),
        StdDuration::from_secs(5 * 60)
    );
    assert_eq!(
        compute_sleep_time(NaiveTime::from_hms(23, 50, 50)),
        StdDuration::from_secs(4 * 60 + 10)
    );
    assert_eq!(
        compute_sleep_time(NaiveTime::from_hms(23, 59, 59)),
        StdDuration::from_secs(1)
    );
}

async fn do_step(ctx: &mut Context<'_>, begin: &NaiveDateTime, end: &NaiveDateTime) -> Result<()> {
    let passed = ctx
        .config
        .backups
        .iter()
        .filter(|x| x.interval.is_passed(&begin, end))
        .collect::<Vec<_>>();

    if !passed.is_empty() {
        trace!(
            "those settings will be used to backup {:?}",
            passed.iter().map(|x| &x.name).collect::<Vec<_>>()
        );
        let backup_file = backup_to_tmp(ctx).await?;

        let futures = passed
            .into_iter()
            .map(|backup| Ok(save_backup(backup_file.try_clone()?, end, backup)))
            .collect::<Result<Vec<_>, Error>>()?;
        join_all(futures).await;
    } else {
        trace!("nothing to do for this step.")
    }

    Ok(())
}

async fn backup_to_tmp(ctx: &mut Context<'_>) -> Result<StdFile> {
    for cmd in &ctx.config.commands_before {
        ctx.send_command(cmd).await?;
    }

    let save_dir = ctx.config.save_dir.clone();
    let tar_file = asyncify(|| {
        let mut file = tempfile::tempfile()?;
        let mut tar = ::tar::Builder::new(BufWriter::new(&mut file));
        // add config file
        let save_dir = save_dir;
        append_dir_all_sorted(&mut tar, "".as_ref(), save_dir.as_path())?;
        tar.finish()?;
        drop(tar);
        file.flush()?;
        Ok(file)
    })
    .await
    .context("saving to temporal tar file.")?;

    for cmd in &ctx.config.commands_after {
        ctx.send_command(cmd).await?;
    }
    Ok(tar_file)
}

async fn save_backup(backup_tar: StdFile, now: &NaiveDateTime, config: &BackupSetting) {
    if let Some(err) = do_save_backup(backup_tar, now, config).await.err() {
        error!(
            "error during backing up for {} at {}: {}",
            config.name, now, err
        )
    }
}

async fn do_save_backup(
    backup_tar: StdFile,
    now: &NaiveDateTime,
    config: &BackupSetting,
) -> Result<()> {
    let mut backup_tar = File::from_std(backup_tar);
    let cfg_name = &config.name;
    let directory = &config.directory;
    tokio::fs::create_dir_all(&directory)
        .await
        .context("back up directory creation")?;

    //let time_for_save = config.interval.get_last_date_until(now);
    let backup_name = now.format("backup-%Y-%m-%d-%H-%M-%S").to_string();
    let tar_path = directory.join(format!("{}.tar", backup_name));
    let files_txt_path = directory.join("files.txt");
    let dot_files_txt_path = directory.join(".files.txt");

    let mut save_tar_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tar_path)
        .await
        .context("creating backup file")?;

    // first, copy backup tar to expected place and close

    tokio::io::AsyncSeekExt::seek(&mut backup_tar, SeekFrom::Start(0))
        .await
        .context("saving backup to file")?;
    tokio::io::copy(&mut backup_tar, &mut save_tar_file)
        .await
        .context("saving backup to file")?;
    tokio::io::AsyncWriteExt::flush(&mut save_tar_file)
        .await
        .context("saving backup to file")?;
    save_tar_file.sync_all().await?;
    drop(save_tar_file);
    trace!("saved to {}", tar_path.display());

    let mut files_txt = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&files_txt_path)
        .await
        .context("opening files.txt")?;

    // second, append to files.txt
    async fn append_to_files(files_txt: &mut File, backup_name: &str) -> Result<()> {
        files_txt.seek(SeekFrom::End(0)).await?;
        files_txt
            .write(format!("\n{}\n", backup_name).as_bytes())
            .await?;
        files_txt.flush().await?;
        files_txt.sync_all().await?;
        Ok(())
    }
    append_to_files(&mut files_txt, &backup_name)
        .await
        .context("appending to files.txt")?;
    trace!("appended to {}", files_txt_path.display());

    // third, remove oldest backup if needed
    async fn read_files_to_vec(files_txt: &mut File) -> Result<Vec<u8>> {
        files_txt.seek(SeekFrom::Start(0)).await?;
        let mut buffer = Vec::new();
        files_txt.read_to_end(&mut buffer).await?;
        Ok(buffer)
    }

    fn parse_files_txt(buffer: &[u8]) -> Vec<&[u8]> {
        buffer
            .split(|b| *b == b'\n')
            .map(|s| s.splitn(2, |b| *b == b'#').next().unwrap())
            .filter(|s| {
                s.into_iter()
                    .any(|b| !matches!(*b, b'\t' | b'\n' | b'\x0C' | b'\r' | b' '))
            })
            .collect::<Vec<_>>()
    }

    let buffer = read_files_to_vec(&mut files_txt)
        .await
        .context("reading files.txt")?;
    let files_lines_v = parse_files_txt(&buffer);

    drop(files_txt);

    let mut files_lines = files_lines_v.as_slice();
    if files_lines.len() > config.max_backups {
        // dot_files_txt_path
        let too_many = files_lines.len() - config.max_backups;
        let to_delete: &[&[u8]];
        {
            let pair = files_lines.split_at(too_many);
            to_delete = pair.0;
            files_lines = pair.1;
        }
        trace!(
            "found too many backups for {}: expected {}, {} more. deleting {}, after {}.",
            cfg_name,
            config.max_backups,
            too_many,
            to_delete.len(),
            files_lines.len(),
        );
        let joined = files_lines.join(&b'\n');

        async fn new_file(
            dot_files_txt_path: &Path,
            files_txt_path: &Path,
            joined: &[u8],
        ) -> Result<()> {
            let mut dot_files_txt = OpenOptions::new()
                .create(true)
                .write(true)
                .open(dot_files_txt_path)
                .await?;

            dot_files_txt.seek(SeekFrom::End(0)).await?;
            dot_files_txt.set_len(0).await?;
            dot_files_txt.write(&joined).await?;
            dot_files_txt.flush().await?;
            dot_files_txt.sync_all().await?;
            drop(dot_files_txt);

            // move files.txt
            remove_file(files_txt_path).await?;
            rename(dot_files_txt_path, files_txt_path).await?;

            Ok(())
        }

        new_file(&dot_files_txt_path, &files_txt_path, &joined)
            .await
            .context("creating new files.txt")?;

        async fn remove_file_allow_not_exist(path: &Path) -> io::Result<()> {
            match remove_file(path).await {
                Ok(_) => Ok(()),
                Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e),
            }
        }

        for name in to_delete {
            match std::str::from_utf8(name) {
                Ok(name) => {
                    trace!("deleting of {}: {}", cfg_name, name);
                    if let Some(err) = try_join_all([
                        remove_file_allow_not_exist(&directory.join(format!("{}.tar", name))),
                        remove_file_allow_not_exist(&directory.join(format!("{}.diff.tar", name))),
                    ])
                    .await
                    .err()
                    {
                        error!("error deleting {} of {}: {}", name, cfg_name, err);
                    }
                }
                Err(e) => {
                    error!(
                        "error deleting {:x?} of {}: invalid utf8 at {}",
                        name,
                        cfg_name,
                        e.valid_up_to()
                    );
                }
            }
        }
    } else {
        trace!(
            "found backups for {}: expected {}, we have {}",
            cfg_name,
            config.max_backups,
            files_lines.len(),
        );
    }

    // forth, replace previously newest backup with patch backup if needed
    if config.backup_mode != BackupMode::Simple && files_lines.len() >= 2 {
        // TODO: impl
    }

    Ok(())
}

struct Context<'a> {
    config: &'a Config,
    connection: Option<Connection>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(config: &'a Config) -> Self {
        Self {
            config,
            connection: None,
        }
    }

    pub(crate) async fn reconnect_rcon(&mut self) -> Result<&mut Connection, rcon::Error> {
        let builder = Connection::builder();
        let builder = match self.config.preset {
            None => builder,
            Some(GamePreset::Minecraft) => builder.enable_minecraft_quirks(true),
        };
        self.connection = Some(
            builder
                .connect(
                    self.config.rcon_address.unwrap(),
                    &self.config.rcon_password,
                )
                .await?,
        );
        Ok(self.connection.as_mut().unwrap())
    }

    pub(crate) async fn send_command(&mut self, command: &str) -> Result<String, rcon::Error> {
        let mut connection = match self.connection.as_mut() {
            Some(s) => s,
            None => self.reconnect_rcon().await?,
        };
        loop {
            match connection.cmd(command).await {
                Ok(s) => return Ok(s),
                Err(rcon::Error::Io(e)) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                    // continue with reconnection
                    connection = self.reconnect_rcon().await?;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

// utility

pub(crate) async fn asyncify<F, T>(f: F) -> std::io::Result<T>
where
    F: FnOnce() -> std::io::Result<T> + Send + 'static,
    T: Send + 'static,
{
    match spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "background task failed",
        )),
    }
}
