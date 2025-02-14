# LLM Bridge

A Rust-based server for managing concurrent client connections to AI services, focusing on Server-Sent Events (SSE) streaming.

## Architecture

- HTTP/TCP server using Axum web framework
- Asynchronous request handling with Tokio
- SSE streaming for real-time AI responses
- Connection management for multiple simultaneous clients

## Features

- Concurrent client connection handling
- SSE streaming from AI providers
- Robust error handling
- Docker containerization
- Automated testing

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
pip install pytest requests sseclient-py
```

## Running

```bash
# Start server
source .env && docker-compose up -d

# Run tests (from test directory with venv activated)
pytest
```
