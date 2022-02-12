# game save backuper

[![a12 maintenance: Slowly](https://api.anatawa12.com/short/a12-slowly-svg)](https://api.anatawa12.com/short/a12-slowly-doc)
<!--
[![Crates.io (latest)](https://img.shields.io/crates/dv/game-save-backuper)](https://crates.io/crates/game-save-backuper)
[![github packages download](https://img.shields.io/badge/packages-download-green?logo=github)](https://github.com/anatawa12/game-save-backuper/pkgs/container/game-save-backuper)
-->

The tool to back up save data of some game server.

# This project is in development, not released yet. the following installation steps may not work.

Currently, this is mainly intended for Minecraft but can be used for other games.

Request to support other game is welcome!
Please read [this section](#adding-game-support).

## How to use

<!--
### using prebuilt binary (recommended for windows)

1. download executable file from [release page.][latest-release].
   * if your PC is windows
      * if your PC is 64bit, please download ``x86_64-pc-windows-gnu``.
      * if your PC is 32bit, please download ``i686-pc-windows-gnu``.
   * if your PC is mac
      * if your PC is M1 or later, please download ``aarch64-apple-darwin``.
      * Otherwise, please download ``x86_64-apple-darwin``.
   * if your PC is linux
      * if your PC is aarch64, please download ``aarch64-unknown-linux-gnu``.
      * if your PC is armv7, please download ``armv7-unknown-linux-gnueabihf``.
      * if your PC is x64, please download ``x86_64-unknown-linux-gnu``.
      * if your PC is x86, please download ``i686-unknown-linux-gnu``.
3. create config file. See [Config format](#config-format).
4. open console and go to the directory config file is in.
5. run the executable file.

[latest-release]: https://github.com/anatawa12/game-save-backuper/releases/latest


### using docker (recommended for macos and linux)
-->

1. get docker image from github packages

       docker pull ghcr.io/anatawa12/game-save-backuper

2. create config file. See [Config format](#config-format).

   Note that you must not set `backup_dir` and `save_dir`.

3. ```bash
   docker run \
     -v '/path/to/your/save/dir:/save' \
     -v '/path/to/your/backups/dir:/backups' \
     ghcr.io/anatawa12/game-save-backuper 
   ```

   to start daemon.


### Config format

```yaml
# choose preset. currently, minecraft are supported. optional.
preset: minecraft
# the path to directory to be backed up.
# This should not be specified if you're using docker
save_dir: /path
# the path to backups directory.
# This should not be specified if you're using docker
backup_dir: /path

# you can back up multiple interval.
backups:
  # name of backup directory
  - name: 5min
    # interval of backup.
    # you can choose from:
    #   5, 10, 15, 20, 30 minutely
    #   1, 2, 4, 6, 12 hourly (every 0 minute)
    #   daily (every 0:00 UTC)
    #   weekly (every monday 0:00 UTC)
    #   1, 2, 3, 4, 6 monthly (every 1st 0:00 UTC)
    #   yearly (every Jan 1st 0:00 UTC)
    interval: 5 minutely
    # the count of backups will be saved.
    # if more than this number of backups are found,
    # the oldest backup will be removed
    max_backups: 12
```

## Adding game support

I think it make this better to support other games.
I don't have enough time to find which game is good to support and to find game information.
That's why it's welcome to send me a request to  support other game.
To request, please open a new issue!
If possible, please let me know the following information to make it easy to support new game.

- whether the game supports rcon to
  - stop and resume auto save if the game have auto save
  - force save world
- the default rcon port number if it supports rcon.
- the command to be sent over rcon to
  - stop and resume auto save if the game have auto save
  - force save world
