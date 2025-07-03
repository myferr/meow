# meow
[![CI](https://github.com/myferr/meow/actions/workflows/release.yml/badge.svg)](https://github.com/myferr/meow/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/myferr/meow?label=latest&style=flat-square)](https://github.com/myferr/meow/releases/latest)
[![License](https://img.shields.io/github/license/myferr/meow?style=flat-square)](LICENSE)

**meow** is a fast, terminal-based IRC client written in Rust.

---

## Installation

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/myferr/meow/main/scripts/install.ps1 | iex
```

### Linux / macOS (Bash)

```bash
curl -fsSL https://raw.githubusercontent.com/myferr/meow/main/scripts/install.sh | bash
```

Make sure `~/.local/bin` is in your `PATH` if you're on a Unix-based (*NIX) system.

---

## Binary Downloads

Prebuilt binaries for **Linux**, **macOS (Intel & Apple Silicon)**, and **Windows** are available on the [Releases page](https://github.com/myferr/meow/releases/latest).

---

## Usage

```bash
meow                                                       # Start interactive experience

# Use the following commands while in interactive mode using meow

/connect <server> <port> <nickname> <tls? (true/false)>    # connect to a server, you can configure a default port, nick, and TLS option if you don't want to fill it out.

/join <#channel>                                           # join a channel
/part <#channel>                                           # leave a channel

/msg <#channel>/<user> <message>                           # send a PRIVMSG to a channel or user.

/quit                                                      # exit the program
```

You can configure defaults in:

```toml
# ~/.meow/config.toml

[irc]
nick = "mycat"
tls = true
port = 6697

[theme]
background = "" # hex code (optional)
foreground = "" # hex code (optional)
muted = "" # hex code (optional)
accent = "" # hex code (optional)
icons = true  # enable Nerd Font icons (optional)

[emojis]
shrug = "¯\\_(ツ)_/¯" # use like :shrug: in /msg commands.
cat = ":3" # use like :cat: in /msg commands.
```
> Windows systems use `%USERPROFILE%/meowconf/config.toml`

The config file is 100% optional. Channel/server are passed via CLI.

---

## Features

 Clean & readable terminal UI
*  Scrollback + input history
* Auto-reconnect
* Nerd Font icons (optional)
* Zero external config required

---

## License

Licensed under the **MIT License**.
See [LICENSE](LICENSE) for details.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md)

## To-do

- [x] TLS support
- [x] SSL support
- [x] Connect to servers
- [x] Join channels
- [x] Leave/part channels
- [x] Send and receive messages
- [x] Display help prompt
- [x] Scrollback (PgUp/PgDn support)
- [x] Input history (arrow key navigation)
- [x] Word wrap & terminal overflow handling
- [x] Auto-reconnect on disconnect
- [x] Graceful error handling (no panics or unwraps)
- [x] Config file support (`~/.meow/config.toml`)
- [x] Adding configurable themes in `~/.meow/config.toml`
- [x] Emoji support via configurable aliases in `~/.meow/config.toml`
- [x] Polish up the program
- [x] Windows and cross-platform builds

## Other stuff :3
* [MAKEFILE.md](MAKEFILE.md)
* [CONTRIBUTING.md](CONTRIBUTING.md)
