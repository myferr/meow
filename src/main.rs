mod app;
mod config;
mod irc_client;
mod ui;

use anyhow::Result;
use app::InputCommand;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, LeaveAlternateScreen},
};
use std::io::{stdout, Write};
use tokio::sync::mpsc;
use crate::config::UserConfig;

#[tokio::main]
async fn main() -> Result<()> {
    // Flush welcome message before UI takes over
    print_welcome_box();
    std::io::stdout().flush()?; // <-- flush to force immediate draw

    // Pause for 2 seconds to allow the user to see the welcome box
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Create communication channels
    let (irc_tx, ui_rx) = mpsc::channel::<String>(100);
    let (ui_tx, input_rx) = mpsc::channel::<InputCommand>(100);

    let config = UserConfig::load();
    let accent_color_hex = config
        .as_ref()
        .and_then(|cfg| cfg.theme.as_ref()?.accent.clone());

    // Spawn IRC logic
    let irc_handle = tokio::spawn({
        let ui_tx = ui_tx.clone();
        let accent_color_hex_for_irc = accent_color_hex.clone();
        async move {
            if let Err(e) = irc_client::run_irc(irc_tx, ui_tx, input_rx, accent_color_hex_for_irc).await {
                eprintln!("IRC client error: {:?}", e);
            }
        }
    });

    // Run the terminal UI
    if let Err(e) = ui::run_ui(ui_tx, ui_rx, accent_color_hex).await {
        eprintln!("UI error: {:?}", e);
    }

    // Clean up terminal
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    irc_handle.await?;

    Ok(())
}

use crossterm::{
    cursor,
    style::{Color, ResetColor, SetForegroundColor},
    ExecutableCommand,
};

pub fn print_welcome_box() {
    let mut out = stdout();

    let lines = [
        "┌──────────────────────────────────────────────────┐",
        "⎹              Welcome to meow IRC Client          ⎹",
        "+--------------------------------------------------+",
        "⎹ Available Commands:                              ⎹",
        "⎹                                                  ⎹",
        "⎹  /connect <server> <port> <nick> <tls?>          ⎹",
        "⎹  /join <#channel>                                ⎹",
        "⎹  /part <#channel>                                ⎹",
        "⎹  /msg <target> <message>                         ⎹",
        "⎹  /quit                                           ⎹",
        "└─────────────────────────────────────────────────┘",
        "",
    ];

    let start_y = 2; // vertical offset

    let _ = out.execute(SetForegroundColor(Color::Cyan));
    for (i, line) in lines.iter().enumerate() {
        let _ = out.execute(cursor::MoveTo(5, start_y + i as u16));
        println!("{}", line);
    }
    let _ = out.execute(ResetColor);
    let _ = out.flush();
}
