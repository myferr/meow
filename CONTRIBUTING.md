# Contributing to meow

Thanks for your interest in contributing to **meow**, a minimal terminal IRC client written in Rust!

We welcome contributions of all kinds — code, bug reports, feature suggestions, or even just design ideas. Here’s how to get started:

---

## Getting Started

1. **Fork the repository** and clone it locally.
   ```bash
   git clone https://github.com/myferr/meow.git
   cd meow
   ```

2. **Install Rust** (if you haven't already):

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **Build the app:**

   ```bash
   cargo build
   ```

4. **Run it:**

   ```bash
   cargo run
   ```

---

## Development Tips

* The UI code is in `src/ui.rs`.
* IRC logic is in `src/irc_client.rs`.
* Commands are handled via `InputCommand` in `src/app.rs`.
* Uses `crossterm` for TUI and `irc` crate for IRC protocol.
* Prefer small, focused PRs.

---

## Contribution Guidelines

* Format code with `cargo fmt`
* Run `cargo clippy` and fix any obvious warnings
* Follow Rust idioms (no panics, unwraps, or TODOs in final code)
* Make UI additions friendly for 80×24 terminals
* Follow [the convential commits specification](https://conventionalcommits.org) (e.g. `feat(ui): add /help box styling`)
* Document any new public functions

---

## Ideas & Tasks

You can contribute by:

* Implementing [features from the To-Do list](./README.md#to-do)
* Improving UX for tiny terminals
* Refactoring the UI loop to improve testability
* Writing tests (especially for non-UI logic)
* Improving UI using ASCII characters


---

## Questions?

* Open an issue
* Leave a discussion thread
* Or just say hi in a GitHub comment :)

---

Thanks again for being awesome and helping meow grow.
