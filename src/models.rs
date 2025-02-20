use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ClaudeStreamApiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ClaudeMessage>,
    pub stream: bool,
}

#[derive(Deserialize, Debug)]
pub struct ClaudeUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStart },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: Delta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDeltaContent,
        usage: ClaudeUsage,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Deserialize, Debug)]
pub struct MessageStart {
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: ClaudeUsage,
}

#[derive(Deserialize, Debug)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
}

#[derive(Deserialize, Debug)]
pub struct Delta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: String,
}

#[derive(Deserialize, Debug)]
pub struct MessageDeltaContent {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}

pub fn parse_sse_line(line: &str) -> Option<StreamEvent> {
    if line.starts_with("data: ") {
        let json = &line["data: ".len()..];
        match serde_json::from_str(json) {
            Ok(event) => Some(event),
            Err(e) => {
                eprintln!("Error parsing SSE data: {}", e);
                None
            }
        }
    } else {
        None
    }
}
