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
use tokio::time::{sleep, Duration};

/// Helper function to connect to the IRC server and start listening for messages.
/// It spawns a new task to handle incoming messages and signals disconnects.
async fn connect_and_listen(
    config: Config,
    irc_tx: Sender<String>,
    input_tx: Sender<InputCommand>, // Sender to send commands back to the main loop (e.g., Disconnected)
) -> Result<Arc<Mutex<Client>>> {
    // Attempt to create a client from the provided configuration.
    let client = Client::from_config(config).await?;
    // Identify the client with the server (e.g., send NICK and USER commands).
    client.identify()?;

    // Wrap the client in Arc<Mutex> for thread-safe sharing across tasks.
    let client_arc = Arc::new(Mutex::new(client));
    // Clone for the spawned message processing task.
    let client_clone_for_spawn = Arc::clone(&client_arc);
    let tx_clone_for_spawn = irc_tx.clone();
    let input_tx_clone_for_spawn = input_tx.clone(); // Clone input_tx for the spawned task

    // --- FIX START ---
    // Acquire a lock on the client to get its message stream *here*,
    // before spawning the task. If this fails, connect_and_listen should return an error.
    let stream = {
        let mut locked_client = client_clone_for_spawn.lock().await;
        locked_client.stream()? // Propagate the error if stream() fails
    };
    // --- FIX END ---

    // Spawn a new asynchronous task to continuously read messages from the IRC stream.
    tokio::spawn(async move {
        let mut stream = stream; // Use the stream obtained above
                                 // Loop to process each message received from the IRC server.
        while let Some(message_result) = stream.next().await {
            match message_result {
                Ok(message) => {
                    // Process different IRC commands and send formatted messages to the UI.
                    match &message.command {
                        Command::PING(server, _) => {
                            let _ = tx_clone_for_spawn
                                .send(format!("*** Ping: {}", server))
                                .await;
                        }
                        Command::JOIN(channel, _, _) => {
                            let prefix = message
                                .prefix
                                .as_ref()
                                .map(|p| p.to_string())
                                .unwrap_or_default();
                            let _ = tx_clone_for_spawn
                                .send(format!("*** {} joined {}", prefix, channel))
                                .await;
                        }
                        Command::PRIVMSG(target, msg) => {
                            // Handle CTCP messages (e.g., ACTION, VERSION)
                            if msg.starts_with('\x01') && msg.ends_with('\x01') {
                                let ctcp = &msg[1..msg.len() - 1];
                                let sender = message.source_nickname().unwrap_or("unknown");
                                let _ = tx_clone_for_spawn
                                    .send(format!("(CTCP) {}: {}", sender, ctcp))
                                    .await;
                            } else if let Some(sender) = message.source_nickname() {
                                // Format regular private messages or channel messages.
                                let formatted = if target.starts_with('#') {
                                    format!("<{}> {}", sender, msg)
                                } else {
                                    format!("<{}->You> {}", sender, msg)
                                };
                                let _ = tx_clone_for_spawn.send(formatted).await;
                            }
                        }
                        Command::NOTICE(target, msg) => {
                            let _ = tx_clone_for_spawn
                                .send(format!("(notice to {}): {}", target, msg))
                                .await;
                        }
                        Command::Raw(cmd, params) if cmd == "MODE" => {
                            let params_str = params.join(" ");
                            let _ = tx_clone_for_spawn
                                .send(format!("*** Mode: {}", params_str))
                                .await;
                        }
                        Command::PART(channel, _) => {
                            let _ = tx_clone_for_spawn
                                .send(format!("*** Left channel {}", channel))
                                .await;
                        }
                        Command::QUIT(reason) => {
                            let reason_str = reason
                                .as_ref()
                                .map(|r| r.to_string())
                                .unwrap_or("Quit".into());
                            if let Some(sender) = message.source_nickname() {
                                let _ = tx_clone_for_spawn
                                    .send(format!("*** {} quit: {}", sender, reason_str))
                                    .await;
                            }
                        }
                        Command::PONG(server, _) => {
                            let _ = tx_clone_for_spawn
                                .send(format!("*** Pong: {}", server))
                                .await;
                        }
                        Command::Response(_code, params) => {
                            // Handle numeric IRC responses (e.g., welcome messages, MOTD).
                            let code = params.get(0).cloned().unwrap_or_default();
                            let msg = params.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
                            let display = match code.as_str() {
                                "001" => format!("*** Welcome: {}", msg),
                                "375" | "372" | "376" => format!("*** MOTD: {}", msg),
                                _ => format!("*** {}: {}", code, msg),
                            };
                            let _ = tx_clone_for_spawn.send(display).await;
                        }
                        _ => {
                            // Log unhandled IRC commands for debugging.
                            let _ = tx_clone_for_spawn
                                .send(format!("*** Unhandled: {:?}", message.command))
                                .await;
                        }
                    }
                }
                Err(e) => {
                    // If an error occurs while receiving a message, it indicates a disconnect.
                    let _ = tx_clone_for_spawn
                        .send(format!(
                            "Error receiving message: {}. Signaling disconnect...",
                            e
                        ))
                        .await;
                    let _ = input_tx_clone_for_spawn
                        .send(InputCommand::Disconnected)
                        .await; // Signal disconnect to main loop
                    return; // Exit the spawned task as the stream is broken
                }
            }
        }
        // If the stream gracefully closes (e.g., server shutdown without error), also signal disconnect.
        let _ = tx_clone_for_spawn
            .send("*** IRC stream closed. Signaling disconnect...".into())
            .await;
        let _ = input_tx_clone_for_spawn
            .send(InputCommand::Disconnected)
            .await; // Signal disconnect
    });

    Ok(client_arc) // Return the client only if stream acquisition and task spawning were successful
}

/// Runs the IRC client logic, handling connect, join, messaging, and receiving.
/// This function now also manages auto-reconnection.
pub async fn run_irc(
    irc_tx: Sender<String>, // Sender for messages to be displayed in the UI
    input_tx: Sender<InputCommand>, // Sender for commands to the IRC client (e.g., from UI input)
    mut input_rx: Receiver<InputCommand>, // Receiver for commands from the UI
) -> Result<()> {
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
                                match connect_and_listen(config.clone(), irc_tx.clone(), input_tx.clone()).await {
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
                                    let message_clone = message.clone();

                                    tokio::spawn(async move {
                                        let locked = client.lock().await;
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
                                        let message_clone = message.clone();

                                        tokio::spawn(async move {
                                            let locked = client.lock().await;
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
                                        match connect_and_listen(config_to_reconnect.clone(), irc_tx.clone(), input_tx.clone()).await {
                                            Ok(new_client) => {
                                                irc_tx.send(format!("*** Reconnected successfully!")).await?;
                                                client_opt = Some(new_client); // Set the new client

                                                // If a channel was previously joined, attempt to re-join it.
                                                if let Some(channel) = &current_channel {
                                                    let client_rejoin = Arc::clone(client_opt.as_ref().unwrap());
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
