#[derive(Debug)]
pub enum InputCommand {
    Connect {
        server: String,
        port: u16,
        nick: String,
    },
    SendMessage {
        target: String,
        message: String,
    },
    JoinChannel(String),
    PartChannel(String),
    Quit,
    SendPlainMessage(String),
}
