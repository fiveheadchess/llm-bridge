use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct ClaudeStreamApiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ClaudeMessage>,
    pub stream: bool,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: Delta },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaContent },
}

#[derive(Deserialize, Debug)]
pub struct Delta {
    pub text: String,
}

#[derive(Deserialize, Debug)]
pub struct MessageDeltaContent {
    pub stop_reason: Option<String>,
}
