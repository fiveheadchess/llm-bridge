# LLM Bridge

A Rust-based server for managing concurrent client connections to AI services, focusing on protocol simplicity.

Right now this is in a pre-alpha state and is just being used for a POC.
Right now Websockets are not supported, the protocol is a custom TCP sequence where a JSON request
is received from the client with a prompt, and a message from the LLM is streamed back in chunks.

```
 Client          Server          Claude API
  |               |                |
  |--JSON REQ---->|                |
  |               |---API REQ----->|
  |<--STREAM-----|<---STREAM------|
  |<--STREAM-----|<---STREAM------|
  |<---DONE------|<----DONE-------|
  |    CLOSE     |                |

```

## Architecture

- HTTP/TCP server using Axum web framework
- Asynchronous request handling with Tokio
- SSE streaming for real-time AI responses
- Connection management for multiple simultaneous clients

## Features

- overly simple TCP connection handling

## Planned Improvements

- Client authentication/authorization
- Rate limiting and request throttling
- Connection persistence (WebSocket support)
- Multi-model support
- Monitoring and metrics (Prometheus integration)
- Request queueing and prioritization
- Response caching
- Failover handling

## Setup

1. Create environment file:

```bash
echo "ANTHROPIC_API_KEY=your_key_here" > .env
```

2. Set up Python test environment:

```bash
# Create venv in test directory
cd test
python3 -m venv venv
source venv/bin/activate  # On Windows: .\venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt
```

## Running

```bash
# Start server
source .env && docker-compose up -d

# Run tests (from test directory with venv activated)
pytest
```
