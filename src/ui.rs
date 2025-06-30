use crate::app::InputCommand;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{Attribute, Color, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::collections::VecDeque;
use std::io::{stdout, Write};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::Duration;
use unicode_segmentation::UnicodeSegmentation;

pub async fn run_ui(
    input_tx: Sender<InputCommand>,
    mut irc_rx: Receiver<String>,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let mut input = String::new();
    let mut messages: VecDeque<Vec<String>> = VecDeque::with_capacity(100);
    let mut scroll_offset: usize = 0;
    let mut input_history: Vec<String> = Vec::new();
    let mut input_history_index: Option<usize> = None;

    let max_width = 80;
    let left_padding = 2;
    let max_height = 20;

    fn format_message(msg: &str, max_width: usize, left_padding: usize) -> Vec<String> {
        let available_width = max_width.saturating_sub(left_padding);
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_len = 0;

        for grapheme in UnicodeSegmentation::graphemes(msg, true) {
            let width = grapheme.chars().count();
            if current_len + width > available_width {
                lines.push(format!(
                    "{:width$}{}",
                    "",
                    current_line,
                    width = left_padding
                ));
                current_line.clear();
                current_len = 0;
            }
            current_line.push_str(grapheme);
            current_len += width;
        }
        if !current_line.is_empty() {
            lines.push(format!(
                "{:width$}{}",
                "",
                current_line,
                width = left_padding
            ));
        }
        lines
    }

    execute!(stdout, Clear(ClearType::All))?;
    let lines = [
        "╭────────────────────────────────────────────────────────────╮",
        "│              \x1b[1mWelcome to meow IRC Client\x1b[0m              │",
        "├────────────────────────────────────────────────────────────┤",
        "│  \x1b[3mAvailable Commands:\x1b[0m                                  │",
        "│                                                            │",
        "│  \x1b[1m/connect <server> <port> <nick>\x1b[0m                 │",
        "│  \x1b[1m/join <#channel>\x1b[0m                                │",
        "│  \x1b[1m/part <#channel>\x1b[0m                                │",
        "│  \x1b[1m/msg <target> <message>\x1b[0m                         │",
        "│  \x1b[1m/quit\x1b[0m                                           │",
        "╰────────────────────────────────────────────────────────────╯",
        "",
        "Press \x1b[1mEnter\x1b[0m to continue...",
    ];

    execute!(stdout, SetForegroundColor(Color::Cyan))?;
    let mut y = 2;
    for line in lines.iter() {
        for wrapped_line in format_message(line, max_width, 0) {
            execute!(stdout, cursor::MoveTo(left_padding as u16, y))?;
            writeln!(stdout, "{}", wrapped_line)?;
            y += 1;
        }
    }
    execute!(stdout, ResetColor)?;
    stdout.flush()?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Enter {
                    break;
                }
            }
        }
    }

    execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;
    stdout.flush()?;

    let mut running = true;
    while running {
        while let Ok(msg) = irc_rx.try_recv() {
            if messages.len() == 100 {
                messages.pop_front();
            }
            messages.push_back(format_message(&msg, max_width, left_padding));
        }

        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;
        execute!(
            stdout,
            SetForegroundColor(Color::Blue),
            SetAttribute(Attribute::Bold)
        )?;
        writeln!(
            stdout,
            "{}╭─ meow IRC Client ── Type /help for commands. ESC to quit ─╮",
            " ".repeat(left_padding)
        )?;
        execute!(stdout, ResetColor)?;

        let flat_messages: Vec<String> = messages.iter().flat_map(|v| v.clone()).collect();
        let start = if flat_messages.len() > max_height + scroll_offset {
            flat_messages.len() - max_height - scroll_offset
        } else {
            0
        };
        let end = flat_messages.len().saturating_sub(scroll_offset);

        for (i, msg) in flat_messages.iter().take(end).skip(start).enumerate() {
            execute!(stdout, cursor::MoveTo(0, (i + 2) as u16))?;
            writeln!(stdout, "{}", msg)?;
        }

        execute!(stdout, cursor::MoveTo(0, (max_height + 2) as u16))?;
        writeln!(stdout)?;
        execute!(
            stdout,
            SetForegroundColor(Color::Green),
            SetAttribute(Attribute::Bold)
        )?;
        for line in format_message(&format!("❯ {}", input), max_width, left_padding) {
            writeln!(stdout, "{}", line)?;
        }
        execute!(stdout, ResetColor)?;
        stdout.flush()?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => {
                        input.push(c);
                        input_history_index = None;
                    }
                    KeyCode::Backspace => {
                        input.pop();
                        input_history_index = None;
                    }
                    KeyCode::Enter => {
                        if !input.trim().is_empty() {
                            input_history.push(input.clone());
                        }
                        input_history_index = None;
                        scroll_offset = 0;

                        let user_msg = format!("You: {}", input);
                        messages.push_back(format_message(&user_msg, max_width, left_padding));

                        if input.starts_with('/') {
                            let mut parts = input.trim().splitn(2, ' ');
                            let cmd = parts.next().unwrap_or("");
                            let arg = parts.next().unwrap_or("");

                            match cmd {
                                "/connect" => {
                                    let mut args = arg.split_whitespace();
                                    let server = args.next().unwrap_or("").to_string();
                                    let port =
                                        args.next().unwrap_or("6697").parse().unwrap_or(6697);
                                    let nick = args.next().unwrap_or("meow").to_string();
                                    let tls = args.next().map_or(true, |v| v == "true" || v == "1");
                                    input_tx
                                        .send(InputCommand::Connect {
                                            server,
                                            port,
                                            nick,
                                            tls,
                                        })
                                        .await?;
                                }
                                "/join" => {
                                    input_tx
                                        .send(InputCommand::JoinChannel(arg.to_string()))
                                        .await?;
                                }
                                "/part" => {
                                    input_tx
                                        .send(InputCommand::PartChannel(arg.to_string()))
                                        .await?;
                                }
                                "/msg" => {
                                    let mut msg_parts = arg.splitn(2, ' ');
                                    if let (Some(target), Some(message)) =
                                        (msg_parts.next(), msg_parts.next())
                                    {
                                        input_tx
                                            .send(InputCommand::SendMessage {
                                                target: target.to_string(),
                                                message: message.to_string(),
                                            })
                                            .await?;
                                    }
                                }
                                "/quit" => {
                                    input_tx.send(InputCommand::Quit).await?;
                                    running = false;
                                }
                                "/help" => {
                                    let help_lines = [
                                        "╭───────────────────────────────────────────────╮",
                                        "│                   Help Menu                  │",
                                        "├───────────────────────────────────────────────┤",
                                        "│ /connect <server> [port] [nick] [tls]        │",
                                        "│ /join <channel>                              │",
                                        "│ /part <channel>                              │",
                                        "│ /msg <target> <message>                      │",
                                        "│ /quit                                        │",
                                        "╰───────────────────────────────────────────────╯",
                                    ];
                                    for line in help_lines {
                                        messages.push_back(format_message(
                                            line,
                                            max_width,
                                            left_padding,
                                        ));
                                    }
                                }
                                _ => {
                                    let unknown = format!("Unknown command: {}", cmd);
                                    messages.push_back(format_message(
                                        &unknown,
                                        max_width,
                                        left_padding,
                                    ));
                                }
                            }
                        }

                        input.clear();
                    }
                    KeyCode::Esc => {
                        input_tx.send(InputCommand::Quit).await?;
                        running = false;
                    }
                    KeyCode::PageUp => {
                        scroll_offset += 5;
                        if scroll_offset > flat_messages.len().saturating_sub(1) {
                            scroll_offset = flat_messages.len().saturating_sub(1);
                        }
                    }
                    KeyCode::PageDown => {
                        scroll_offset = scroll_offset.saturating_sub(5);
                    }
                    KeyCode::Up => {
                        if input_history.is_empty() {
                            continue;
                        }
                        match input_history_index {
                            Some(0) => {}
                            Some(i) => input_history_index = Some(i - 1),
                            None => {
                                input_history_index = Some(input_history.len().saturating_sub(1))
                            }
                        }
                        if let Some(i) = input_history_index {
                            if let Some(entry) = input_history.get(i) {
                                input = entry.clone();
                            }
                        }
                    }
                    KeyCode::Down => {
                        if input_history.is_empty() {
                            continue;
                        }
                        match input_history_index {
                            Some(i) if i + 1 < input_history.len() => {
                                input_history_index = Some(i + 1);
                                if let Some(entry) = input_history.get(i + 1) {
                                    input = entry.clone();
                                }
                            }
                            _ => {
                                input_history_index = None;
                                input.clear();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;
    disable_raw_mode()?;
    Ok(())
}
