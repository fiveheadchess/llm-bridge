import pytest
import requests
import sseclient
import json
import os

BASE_URL = "http://localhost:8080"


def test_stream_response():
    request_data = {
        "model": "claude-3-sonnet-20240229",
        "max_tokens": 1000,
        "messages": [{"role": "user", "content": "Say hello"}],
        "stream": True,
    }

    response = requests.post(
        f"{BASE_URL}/ai",
        json=request_data,
        stream=True,
        headers={"Accept": "text/event-stream"},
    )

    assert response.status_code == 200

    client = sseclient.SSEClient(response)
    received_content = []

    for event in client.events():
        received_content.append(event.data)
        if "Stop reason" in event.data:
            break

    assert len(received_content) > 0
    assert not any("error" in str(content).lower() for content in received_content)


def test_invalid_request():
    request_data = {
        "model": "claude-3-sonnet-20240229",
        "max_tokens": 1000,
        "messages": [],  # Empty messages should trigger 400
        "stream": True,
    }

    response = requests.post(
        f"{BASE_URL}/ai",
        json=request_data,
        stream=True,
        headers={"Accept": "text/event-stream"},
    )

    assert response.status_code == 400
