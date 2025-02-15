use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}
#[derive(Serialize)]
pub struct ClaudeApiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ClaudeMessage>,
}

#[derive(Serialize, Deserialize)]
pub struct ClaudeStreamApiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ClaudeMessage>,
    pub stream: bool,
}

#[derive(Deserialize, Debug)]
pub struct ClaudeContentItem {
    text: String,
    #[serde(rename = "type")]
    content_type: String,
}

#[derive(Deserialize, Debug)]
pub struct ClaudeUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct ClaudeApiResponse {
    pub content: Vec<ClaudeContentItem>,
    pub id: String,
    pub model: String,
    pub role: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    #[serde(rename = "type")]
    pub response: String,
    pub usage: ClaudeUsage,
}

#[derive(Deserialize, Debug)]
pub struct ClaudeErrorDetails {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

#[derive(Deserialize, Debug)]
pub struct ClaudeApiError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub error: ClaudeErrorDetails,
}

// Add these to modules.rs

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

// Update the ClaudeApiRequest to include stream option
#[derive(Serialize, Deserialize)]
pub struct ClaudeStreamApiRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ClaudeMessage>,
    pub stream: bool,
}

// Add thfunction to parse SSE events
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
