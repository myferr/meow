# meow

**meow** is a terminal-based IRC (**I**nternet **R**elay **C**hat) client written in Rust, with built-in support for TLS and SSL.

[Join the waitlist!](https://meow-irc.vercel.app)

---

## Disclaimer

This project is currently in active development. Please do not use it in production yet, the project is not ready for daily usage. You're welcome to contribute, thank you!

---

## Contributing
Contributions to [meow](https://github.com/myferr/meow) are welcome and encouraged! Before you contribute please read [CONTRIBUTING.md](CONTRIBUTING.md).

---

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
- [ ] Clean shutdown (e.g. Ctrl+C support)
- [ ] Windows and cross-platform builds
