import pytest
import socket
import json
import os
from dotenv import load_dotenv

load_dotenv()

SERVER_HOST = "localhost"
SERVER_PORT = 8080


def send_and_receive(message, streaming=False):
    """Helper function to send a message and get response"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect((SERVER_HOST, SERVER_PORT))
        s.sendall(f"{message}\n".encode())

        if not streaming:
            # For echo mode, read until connection closes
            response = b""
            while True:
                chunk = s.recv(1024)
                if not chunk:
                    break
                response += chunk
            return response.decode()
        else:
            # For LLM mode, read until we get a complete response
            response = ""
            while True:
                chunk = s.recv(1024).decode()
                response += chunk
                if "TEST_RESPONSE_MARKER" in response:
                    break
            return response


def test_echo_mode():
    """Test echo mode returns exact message"""
    test_message = {"type": "ECHO", "message": "Hello, World!"}
    response = send_and_receive(json.dumps(test_message))
    assert json.loads(response) == test_message


def test_llm_mode():
    """Test LLM mode returns streaming response"""
    if not os.getenv("ANTHROPIC_API_KEY"):
        pytest.skip("ANTHROPIC_API_KEY not set")

    request = {"type": "LLM", "prompt": "Say exactly this: TEST_RESPONSE_MARKER"}

    response = send_and_receive(json.dumps(request), streaming=True)
    assert "TEST_RESPONSE_MARKER" in response
    assert "error" not in response.lower()
