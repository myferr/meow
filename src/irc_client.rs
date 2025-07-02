use crate::app::InputCommand;
use crate::config::{parse_color, UserConfig};
use anyhow::Result;
use futures_util::stream::StreamExt;
use irc::client::prelude::*;
use std::sync::Arc;
use tokio::select;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};
use tokio::time::{sleep, Duration};

/// Runs the IRC client logic, handling connect, join, messaging, and receiving.
/// This function now also manages auto-reconnection.
pub async fn run_irc(
    irc_tx: Sender<String>, // Sender for messages to be displayed in the UI
    input_tx: Sender<InputCommand>, // Sender for commands to the IRC client (e.g., from UI input)
    mut input_rx: Receiver<InputCommand>, // Receiver for commands from the UI
    accent_color_hex: Option<String>,
) -> Result<()> {
    let user_config = UserConfig::load().unwrap_or_default();
    let accent_color = accent_color_hex.and_then(|hex| parse_color(&hex));
    let mut client_opt: Option<Arc<Mutex<Client>>> = None; // Stores the active IRC client
    let mut current_channel: Option<String> = None; // Stores the currently joined channel (for rejoining)
    let mut last_config: Option<Config> = None; // Stores the configuration for the last successful connection

    loop {
        // Use tokio::select to concurrently listen for new commands and handle them.
        select! {
            maybe_cmd = input_rx.recv() => {
                match maybe_cmd {
                    Some(cmd) => {
                        match cmd {
                            InputCommand::Connect { server, port, nick, tls } => {
                                // Create a new IRC client configuration.
                                let config = Config {
                                    nickname: Some(nick.clone()),
                                    username: Some(nick.clone()),
                                    realname: Some("meow IRC Client".into()),
                                    server: Some(server.clone()),
                                    port: Some(port),
                                    use_tls: Some(tls),
                                    ..Default::default()
                                };

                                // Attempt to connect and start listening using the helper function.
                                match connect_and_listen(config.clone(), irc_tx.clone(), input_tx.clone(), accent_color.clone()).await {
                                    Ok(client) => {
                                        // On successful connection, update client_opt and store the config.
                                        irc_tx.send(format!(
                                            "Connected to {}:{} as {} {} TLS",
                                            server,
                                            port,
                                            nick,
                                            if tls { "with" } else { "without" }
                                        )).await?;
                                        client_opt = Some(client);
                                        last_config = Some(config); // Store this config for potential reconnects
                                    }
                                    Err(e) => {
                                        // Report connection errors to the UI.
                                        let _ = irc_tx.send(format!("Error connecting: {}", e)).await;
                                    }
                                }
                            }

                            InputCommand::SendMessage { target, message } => {
                                // If connected, send the message.
                                if let Some(client) = &client_opt {
                                    let client = Arc::clone(client);
                                    let tx_clone = irc_tx.clone();
                                    let target_clone = target.clone();
                                    let mut processed_message = message.clone();

                                    if let Some(emojis_config) = &user_config.emojis {
                                        for (alias, emoji) in &emojis_config.aliases {
                                            processed_message = processed_message.replace(&format!(":{}:", alias), emoji);
                                        }
                                    }

                                    tokio::spawn(async move {
                                        let locked = client.lock().await;
                                        if let Err(e) = locked.send_privmsg(&target_clone, &processed_message) {
                                            let _ = tx_clone.send(format!("Error sending to {}: {}", target_clone, e)).await;
                                        } else {
                                            let color_code = if let Some(crossterm::style::Color::Rgb { r, g, b }) = accent_color {
                                                format!("38;2;{};{};{}", r, g, b)
                                            } else {
                                                "38;2;128;0;128".to_string() // Default purple
                                            };
                                            let _ = tx_clone.send(format!("\x1b[1m\x1b[{}m<You->{}>\x1b[0m {}", color_code, target_clone, processed_message)).await;
                                        }
                                    });
                                } else {
                                    irc_tx.send("Not connected. Use /connect first.".into()).await?;
                                }
                            }

                            InputCommand::JoinChannel(channel) => {
                                // If connected, join the specified channel.
                                if let Some(client) = &client_opt {
                                    let client = Arc::clone(client);
                                    let tx_clone = irc_tx.clone();
                                    let channel_clone = channel.clone();

                                    tokio::spawn(async move {
                                        let locked = client.lock().await;
                                        if let Err(e) = locked.send_join(&channel_clone) {
                                            let _ = tx_clone.send(format!("Error joining {}: {}", channel_clone, e)).await;
                                        } else {
                                            let _ = tx_clone.send(format!("*** Joined {}", channel_clone)).await;
                                        }
                                    });

                                    current_channel = Some(channel); // Update the current channel
                                } else {
                                    irc_tx.send("Not connected. Use /connect first.".into()).await?;
                                }
                            }

                            InputCommand::PartChannel(channel) => {
                                // If connected, part the specified channel.
                                if let Some(client) = &client_opt {
                                    let client = Arc::clone(client);
                                    let tx_clone = irc_tx.clone();
                                    let channel_clone = channel.clone();

                                    tokio::spawn(async move {
                                        let locked = client.lock().await;
                                        if let Err(e) = locked.send_part(&channel_clone) {
                                            let _ = tx_clone.send(format!("Error parting {}: {}", channel_clone, e)).await;
                                        } else {
                                            let _ = tx_clone.send(format!("*** Left {}", channel_clone)).await;
                                        }
                                    });

                                    // If the parted channel was the current one, clear it.
                                    if current_channel.as_ref() == Some(&channel) {
                                        current_channel = None;
                                    }
                                } else {
                                    irc_tx.send("Not connected. Use /connect first.".into()).await?;
                                }
                            }

                            InputCommand::Quit => {
                                // If connected, send a quit message and then exit the loop.
                                if let Some(client) = &client_opt {
                                    let locked = client.lock().await;
                                    let _ = locked.send_quit("Bye!");
                                }
                                break; // Exit the main loop, terminating the client
                            }

                            InputCommand::SendPlainMessage(message) => {
                                // If in a channel, send a plain message to it.
                                if let Some(channel) = &current_channel {
                                    if let Some(client) = &client_opt {
                                        let client = Arc::clone(client);
                                        let tx_clone = irc_tx.clone();
                                        let channel_clone = channel.clone();
                                        let mut processed_message = message.clone();

                                        if let Some(emojis_config) = &user_config.emojis {
                                            for (alias, emoji) in &emojis_config.aliases {
                                                processed_message = processed_message.replace(&format!(":{}:", alias), emoji);
                                            }
                                        }

                                        tokio::spawn(async move {
                                            let locked = client.lock().await;
                                            if let Err(e) = locked.send_privmsg(&channel_clone, &processed_message) {
                                                let _ = tx_clone.send(format!("Error sending: {}", e)).await;
                                            } else {
                                                let color_code = if let Some(crossterm::style::Color::Rgb { r, g, b }) = accent_color {
                                                    format!("38;2;{};{};{}", r, g, b)
                                                } else {
                                                    "38;2;128;0;128".to_string() // Default purple
                                                };
                                                let _ = tx_clone.send(format!("\x1b[1m\x1b[{}m<You ({}) :>\x1b[0m {}", color_code, channel_clone, processed_message)).await;
                                            }
                                        });
                                    }
                                } else {
                                    irc_tx.send("Not in a channel. Use /join.".into()).await?;
                                }
                            }

                            InputCommand::Disconnected => {
                                // Handle the disconnect signal from the message processing task.
                                irc_tx.send("*** Disconnected from IRC server. Attempting to reconnect...".into()).await?;
                                client_opt = None; // Invalidate the current client

                                if let Some(config_to_reconnect) = last_config.clone() {
                                    let mut reconnect_attempts = 0;
                                    loop {
                                        reconnect_attempts += 1;
                                        irc_tx.send(format!("Attempting reconnection #{}...", reconnect_attempts)).await?;
                                        // Implement exponential backoff with a maximum delay.
                                        let delay_secs = (5 * reconnect_attempts).min(60); // Cap delay at 60 seconds
                                        sleep(Duration::from_secs(delay_secs as u64)).await;

                                        // Attempt to reconnect using the stored configuration.
                                        match connect_and_listen(config_to_reconnect.clone(), irc_tx.clone(), input_tx.clone(), accent_color.clone()).await {
                                            Ok(new_client) => {
                                                irc_tx.send(format!("*** Reconnected successfully!")).await?;
                                                client_opt = Some(new_client); // Set the new client

                                                // If a channel was previously joined, attempt to re-join it.
                                                if let Some(channel) = &current_channel {
                                                    if let Some(client_ref) = client_opt.as_ref() {
                                                        let client_rejoin = Arc::clone(client_ref);
                                                        let tx_rejoin = irc_tx.clone();
                                                        let channel_rejoin = channel.clone();
                                                        tokio::spawn(async move {
                                                            let locked = client_rejoin.lock().await;
                                                            if let Err(e) = locked.send_join(&channel_rejoin) {
                                                                let _ = tx_rejoin.send(format!("Error rejoining {}: {}", channel_rejoin, e)).await;
                                                            } else {
                                                                let _ = tx_rejoin.send(format!("*** Rejoined {}", channel_rejoin)).await;
                                                            }
                                                        });
                                                    }
                                                }
                                                break; // Break out of the reconnection loop
                                            }
                                            Err(e) => {
                                                // Report reconnection attempt failures.
                                                irc_tx.send(format!("Error during reconnection attempt #{}: {}", reconnect_attempts, e)).await?;
                                                // Continue to the next attempt after the delay.
                                            }
                                        }
                                    }
                                } else {
                                    // If no previous config, cannot reconnect automatically.
                                    irc_tx.send("Cannot reconnect: No previous connection configuration found.".into()).await?;
                                }
                            }
                        }
                    },
                    None => break, // Input channel closed; exit the main loop.
                }
            }
        }
    }

    Ok(())
}

async fn connect_and_listen(
    config: Config,
    irc_tx: Sender<String>,
    input_tx: Sender<InputCommand>,
    accent_color: Option<crossterm::style::Color>,
) -> Result<Arc<Mutex<Client>>> {
    let mut client = Client::from_config(config).await?;
    client.identify()?;

    let client = Arc::new(Mutex::new(client));
    let client_clone = Arc::clone(&client);
    let irc_tx_clone = irc_tx.clone();
    let input_tx_clone = input_tx.clone();

    tokio::spawn(async move {
        let mut stream = match client_clone.lock().await.stream() {
            Ok(s) => s,
            Err(e) => {
                let _ = irc_tx_clone
                    .send(format!("Error getting IRC stream: {}", e))
                    .await;
                let _ = input_tx_clone.send(InputCommand::Disconnected).await;
                return;
            }
        };
        loop {
            select! {
                // Handle IRC messages
                maybe_message = stream.next() => {
                    if let Some(Ok(message)) = maybe_message {
                        match message.command {
                            Command::PRIVMSG(target, msg) => {
                                if let Some(ref prefix) = message.prefix {
                                    let prefix_str = prefix.to_string();
                                    let parts: Vec<&str> = prefix_str.split('!').collect();
                                    let nick = parts[0];


                                    let color_code = if let Some(crossterm::style::Color::Rgb { r, g, b }) = accent_color {
                                        format!("38;2;{};{};{}", r, g, b)
                                    } else {
                                        "38;2;128;0;128".to_string() // Default purple
                                    };

                                    let _ = irc_tx_clone.send(format!("\x1b[1m\x1b[{}m<{}>\x1b[0m {}", color_code, nick, msg)).await;
                                }
                            }
                            Command::PING(param, _) => {
                                // Respond to PING to keep the connection alive
                                let _ = client_clone.lock().await.send_pong(&param);
                            }
                            Command::ERROR(e) => {
                                let _ = irc_tx_clone.send(format!("IRC Error: {}", e)).await;
                                let _ = input_tx_clone.send(InputCommand::Disconnected).await; // Signal disconnection
                                break; // Exit message processing loop on error
                            }
                            _ => {
                                // For other messages, just display them as is for now.
                                let _ = irc_tx_clone.send(format!("{}", message.to_string())).await;
                            }
                        }
                    } else {
                        // Stream ended, meaning disconnected.
                        let _ = input_tx_clone.send(InputCommand::Disconnected).await; // Signal disconnection
                        break; // Exit message processing loop
                    }
                }
            }
        }
    });

    Ok(client)
}
