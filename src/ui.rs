use crate::app::InputCommand;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    style::{Color, ResetColor, SetForegroundColor},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::collections::VecDeque;
use std::io::{stdout, Write};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::Duration;

pub async fn run_ui(
    input_tx: Sender<InputCommand>,
    mut irc_rx: Receiver<String>,
) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    let mut input = String::new();
    let mut messages: VecDeque<String> = VecDeque::with_capacity(100);
    let max_width = 80; // Maximum width for messages to prevent overflow
    let left_padding = 2; // Left padding for alignment

    // Function to pad and truncate messages
    fn format_message(msg: &str, max_width: usize, left_padding: usize) -> String {
        let available_width = max_width.saturating_sub(left_padding);
        let truncated = if msg.len() > available_width {
            &msg[..available_width]
        } else {
            msg
        };
        format!("{:width$}{}", "", truncated, width = left_padding)
    }

    // Print the welcome box
    execute!(stdout, Clear(ClearType::All))?;
    let lines = [
        "+--------------------------------------------------+",
        "|              Welcome to Rust IRC Client          |",
        "+--------------------------------------------------+",
        "| Available Commands:                              |",
        "|                                                  |",
        "|  /connect <server> <port> <nick>                 |",
        "|  /join <#channel>                                |",
        "|  /part <#channel>                                |",
        "|  /msg <target> <message>                         |",
        "|  /quit                                           |",
        "+--------------------------------------------------+",
        "",
        "Press Enter to continue...",
    ];

    execute!(stdout, SetForegroundColor(Color::Cyan))?;
    for (i, line) in lines.iter().enumerate() {
        execute!(stdout, cursor::MoveTo(left_padding as u16, i as u16 + 2))?;
        writeln!(stdout, "{}", format_message(line, max_width, 0))?;
    }
    execute!(stdout, ResetColor)?;
    stdout.flush()?;

    // Wait for user confirmation
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Enter {
                    break;
                }
            }
        }
    }

    // Clear the screen and start the input prompt
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

        // Display header
        execute!(stdout, SetForegroundColor(Color::Blue))?;
        writeln!(
            stdout,
            "{}Rust IRC Client | Type /help for commands. ESC to quit.",
            " ".repeat(left_padding)
        )?;
        execute!(stdout, ResetColor)?;

        // Display messages
        let height = 20;
        let start = if messages.len() > height {
            messages.len() - height
        } else {
            0
        };
        for (i, msg) in messages.iter().skip(start).enumerate() {
            execute!(stdout, cursor::MoveTo(0, (i + 2) as u16))?;
            writeln!(stdout, "{}", msg)?;
        }

        // Display input prompt
        execute!(stdout, cursor::MoveTo(0, (height + 2) as u16))?;
        writeln!(stdout)?;
        execute!(stdout, SetForegroundColor(Color::Green))?;
        write!(
            stdout,
            "{}> {}",
            " ".repeat(left_padding),
            format_message(&input, max_width - 2, 0)
        )?;
        execute!(stdout, ResetColor)?;
        stdout.flush()?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Enter => {
                        if input.starts_with('/') {
                            let mut parts = input.splitn(2, ' ');
                            let cmd = parts.next().unwrap_or("");
                            let arg = parts.next().unwrap_or("");

                            match cmd {
                                "/connect" => {
                                    let mut args = arg.split_whitespace();
                                    let server = args.next().unwrap_or("").to_string();
                                    let port =
                                        args.next().unwrap_or("6697").parse().unwrap_or(6697);
                                    let nick = args.next().unwrap_or("rusty").to_string();

                                    // Parse TLS option (default true)
                                    let tls = args
                                        .next()
                                        .map(|v| {
                                            matches!(
                                                v.to_lowercase().as_str(),
                                                "true" | "yes" | "1"
                                            )
                                        })
                                        .unwrap_or(true);

                                    let server_clone = server.clone();
                                    let nick_clone = nick.clone();

                                    if server.is_empty() {
                                        messages.push_back(format_message(
                                            "You: Usage: /connect <server> [port] [nick] [tls]",
                                            max_width,
                                            left_padding,
                                        ));
                                    } else {
                                        input_tx
                                            .send(InputCommand::Connect {
                                                server,
                                                port,
                                                nick,
                                                tls,
                                            })
                                            .await?;

                                        messages.push_back(format_message(
                                            &format!(
                                                "You: /connect {} {} {}",
                                                server_clone, port, nick_clone
                                            ),
                                            max_width,
                                            left_padding,
                                        ));
                                    }
                                }
                                "/join" => {
                                    if !arg.is_empty() {
                                        input_tx
                                            .send(InputCommand::JoinChannel(arg.to_string()))
                                            .await?;
                                        messages.push_back(format_message(
                                            &format!("You: /join {}", arg),
                                            max_width,
                                            left_padding,
                                        ));
                                    }
                                }
                                "/part" => {
                                    if !arg.is_empty() {
                                        input_tx
                                            .send(InputCommand::PartChannel(arg.to_string()))
                                            .await?;
                                        messages.push_back(format_message(
                                            &format!("You: /part {}", arg),
                                            max_width,
                                            left_padding,
                                        ));
                                    }
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
                                        messages.push_back(format_message(
                                            &format!("You: /msg {} {}", target, message),
                                            max_width,
                                            left_padding,
                                        ));
                                    }
                                }
                                "/quit" => {
                                    input_tx.send(InputCommand::Quit).await?;
                                    messages.push_back(format_message(
                                        "You: /quit",
                                        max_width,
                                        left_padding,
                                    ));
                                    running = false;
                                }
                                "/help" => {
                                    let help_lines = [
                                        "+---------------------------------------------+",
                                        "|                 Help Menu                   |",
                                        "+---------------------------------------------+",
                                        "| /connect <server> [port] [nick]             |",
                                        "| /join <channel>                             |",
                                        "| /part <channel>                             |",
                                        "| /msg <target> <message>                     |",
                                        "| /quit                                       |",
                                        "+---------------------------------------------+",
                                    ];
                                    messages.push_back(format_message(
                                        "You: /help",
                                        max_width,
                                        left_padding,
                                    ));
                                    for line in help_lines {
                                        messages.push_back(format_message(
                                            &line.to_string(),
                                            max_width,
                                            left_padding,
                                        ));
                                    }
                                }
                                _ => {
                                    messages.push_back(format_message(
                                        &format!("You: Unknown command: {}", cmd),
                                        max_width,
                                        left_padding,
                                    ));
                                }
                            }
                        } else if !input.is_empty() {
                            messages.push_back(format_message(
                                &format!("You: {}", input),
                                max_width,
                                left_padding,
                            ));
                        }
                        input.clear();
                    }
                    KeyCode::Esc => {
                        input_tx.send(InputCommand::Quit).await?;
                        messages.push_back(format_message("You: /quit", max_width, left_padding));
                        running = false;
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
