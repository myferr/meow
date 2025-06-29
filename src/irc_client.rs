use crate::app::InputCommand;
use anyhow::Result;
use futures_util::stream::StreamExt;
use irc::client::prelude::*;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

/// Runs the IRC client logic, handling connect, join, messaging, and receiving.
pub async fn run_irc(irc_tx: Sender<String>, mut input_rx: Receiver<InputCommand>) -> Result<()> {
    let mut client_opt: Option<Arc<Mutex<Client>>> = None;
    let mut current_channel: Option<String> = None;

    loop {
        select! {
            maybe_cmd = input_rx.recv() => {
                match maybe_cmd {
                    Some(cmd) => match cmd {
                        InputCommand::Connect { server, port, nick, tls } => {
                            let config = Config {
                                nickname: Some(nick.clone()),
                                username: Some(nick.clone()),
                                realname: Some("meow IRC Client".into()),
                                server: Some(server.clone()),
                                port: Some(port),
                                use_tls: Some(tls),
                                ..Default::default()
                            };

                            match Client::from_config(config).await {
                                Ok(client) => {
                                    if let Err(e) = client.identify() {
                                        let _ = irc_tx.send(format!("Error identifying client: {}", e)).await;
                                        continue;
                                    }

                                    let client = Arc::new(Mutex::new(client));
                                    let client_clone = Arc::clone(&client);
                                    let tx_clone = irc_tx.clone();

                                    tokio::spawn(async move {
                                        // Lock only to get stream, then drop lock
                                        let stream = {
                                            let mut locked = client_clone.lock().await;
                                            match locked.stream() {
                                                Ok(s) => s,
                                                Err(e) => {
                                                    let _ = tx_clone.send(format!("Error initializing IRC stream: {}", e)).await;
                                                    return;
                                                }
                                            }
                                        };

                                        let mut stream = stream;

                                        while let Some(message_result) = stream.next().await {
                                            match message_result {
                                                Ok(message) => {
                                                    match &message.command {
                                                        Command::PING(server, _) => {
                                                            let _ = tx_clone.send(format!("*** Ping: {}", server)).await;
                                                        }
                                                        Command::JOIN(channel, _, _) => {
                                                            let prefix = message.prefix.as_ref().map(|p| p.to_string()).unwrap_or_default();
                                                            let _ = tx_clone.send(format!("*** {} joined {}", prefix, channel)).await;
                                                        }
                                                        Command::PRIVMSG(target, msg) => {
                                                            if msg.starts_with('\x01') && msg.ends_with('\x01') {
                                                                let ctcp = &msg[1..msg.len() - 1];
                                                                let sender = message.source_nickname().unwrap_or("unknown");
                                                                let _ = tx_clone.send(format!("(CTCP) {}: {}", sender, ctcp)).await;
                                                            } else if let Some(sender) = message.source_nickname() {
                                                                let formatted = if target.starts_with('#') {
                                                                    format!("<{}> {}", sender, msg)
                                                                } else {
                                                                    format!("<{}->You> {}", sender, msg)
                                                                };
                                                                let _ = tx_clone.send(formatted).await;
                                                            }
                                                        }
                                                        Command::NOTICE(target, msg) => {
                                                            let _ = tx_clone.send(format!("(notice to {}): {}", target, msg)).await;
                                                        }
                                                        Command::Raw(cmd, params) if cmd == "MODE" => {
                                                            let params_str = params.join(" ");
                                                            let _ = tx_clone.send(format!("*** Mode: {}", params_str)).await;
                                                        }
                                                        Command::PART(channel, _) => {
                                                            let _ = tx_clone.send(format!("*** Left channel {}", channel)).await;
                                                        }
                                                        Command::QUIT(reason) => {
                                                            let reason_str = reason.as_ref().map(|r| r.to_string()).unwrap_or("Quit".into());
                                                            if let Some(sender) = message.source_nickname() {
                                                                let _ = tx_clone.send(format!("*** {} quit: {}", sender, reason_str)).await;
                                                            }
                                                        }
                                                        Command::PONG(server, _) => {
                                                            let _ = tx_clone.send(format!("*** Pong: {}", server)).await;
                                                        }
                                                        Command::Response(_code, params) => {
                                                            let code = params.get(0).cloned().unwrap_or_default();
                                                            let msg = params.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
                                                            let display = match code.as_str() {
                                                                "001" => format!("*** Welcome: {}", msg),
                                                                "375" | "372" | "376" => format!("*** MOTD: {}", msg),
                                                                _ => format!("*** {}: {}", code, msg),
                                                            };
                                                            let _ = tx_clone.send(display).await;
                                                        }
                                                        _ => {
                                                            let _ = tx_clone.send(format!("*** Unhandled: {:?}", message.command)).await;
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = tx_clone.send(format!("Error receiving message: {}", e)).await;
                                                    break; // Exit on stream error, could reconnect if desired
                                                }
                                            }
                                        }
                                        let _ = tx_clone.send("*** IRC stream closed.".into()).await;
                                    });

                                    irc_tx.send(format!(
                                        "Connected to {}:{} as {} {} TLS",
                                        server,
                                        port,
                                        nick,
                                        if tls { "with" } else { "without" }
                                    )).await?;

                                    client_opt = Some(client);
                                }

                                Err(e) => {
                                    let _ = irc_tx.send(format!("Error connecting: {}", e)).await;
                                }
                            }
                        }

                        InputCommand::SendMessage { target, message } => {
                            if let Some(client) = &client_opt {
                                let client = Arc::clone(client);
                                let tx_clone = irc_tx.clone();
                                let target_clone = target.clone();
                                let message_clone = message.clone();

                                tokio::spawn(async move {
                                    let mut locked = client.lock().await;
                                    if let Err(e) = locked.send_privmsg(&target_clone, &message_clone) {
                                        let _ = tx_clone.send(format!("Error sending to {}: {}", target_clone, e)).await;
                                    } else {
                                        let _ = tx_clone.send(format!("<You->{}> {}", target_clone, message_clone)).await;
                                    }
                                });
                            } else {
                                irc_tx.send("Not connected. Use /connect first.".into()).await?;
                            }
                        }

                        InputCommand::JoinChannel(channel) => {
                            if let Some(client) = &client_opt {
                                let client = Arc::clone(client);
                                let tx_clone = irc_tx.clone();
                                let channel_clone = channel.clone();

                                tokio::spawn(async move {
                                    let mut locked = client.lock().await;
                                    if let Err(e) = locked.send_join(&channel_clone) {
                                        let _ = tx_clone.send(format!("Error joining {}: {}", channel_clone, e)).await;
                                    } else {
                                        let _ = tx_clone.send(format!("*** Joined {}", channel_clone)).await;
                                    }
                                });

                                current_channel = Some(channel);
                            } else {
                                irc_tx.send("Not connected. Use /connect first.".into()).await?;
                            }
                        }

                        InputCommand::PartChannel(channel) => {
                            if let Some(client) = &client_opt {
                                let client = Arc::clone(client);
                                let tx_clone = irc_tx.clone();
                                let channel_clone = channel.clone();

                                tokio::spawn(async move {
                                    let mut locked = client.lock().await;
                                    if let Err(e) = locked.send_part(&channel_clone) {
                                        let _ = tx_clone.send(format!("Error parting {}: {}", channel_clone, e)).await;
                                    } else {
                                        let _ = tx_clone.send(format!("*** Left {}", channel_clone)).await;
                                    }
                                });

                                if current_channel.as_ref() == Some(&channel) {
                                    current_channel = None;
                                }
                            } else {
                                irc_tx.send("Not connected. Use /connect first.".into()).await?;
                            }
                        }

                        InputCommand::Quit => {
                            if let Some(client) = &client_opt {
                                let mut locked = client.lock().await;
                                let _ = locked.send_quit("Bye!");
                            }
                            break;
                        }

                        InputCommand::SendPlainMessage(message) => {
                            if let Some(channel) = &current_channel {
                                if let Some(client) = &client_opt {
                                    let client = Arc::clone(client);
                                    let tx_clone = irc_tx.clone();
                                    let channel_clone = channel.clone();
                                    let message_clone = message.clone();

                                    tokio::spawn(async move {
                                        let mut locked = client.lock().await;
                                        if let Err(e) = locked.send_privmsg(&channel_clone, &message_clone) {
                                            let _ = tx_clone.send(format!("Error sending: {}", e)).await;
                                        } else {
                                            let _ = tx_clone.send(format!("<You ({}):> {}", channel_clone, message_clone)).await;
                                        }
                                    });
                                }
                            } else {
                                irc_tx.send("Not in a channel. Use /join.".into()).await?;
                            }
                        }
                    },
                    None => break, // input channel closed; exit loop
                }
            }
        }
    }

    Ok(())
}
