import pytest
import socket
import json
import os
from dotenv import load_dotenv

load_dotenv()

SERVER_HOST = "localhost"
SERVER_PORT = 8080


def send_and_receive(message):
    """Helper function to send a message and get response"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect((SERVER_HOST, SERVER_PORT))
        s.sendall(f"{message}\n".encode())

        # Read response
        response = b""
        while True:
            chunk = s.recv(1024)
            if not chunk:
                break
            response += chunk

        return response.decode()


def test_echo_mode():
    """Test echo mode returns exact message"""
    os.environ["ECHO_MODE"] = "1"
    test_message = {"message": "Hello, World!"}  # Changed to match EchoRequest struct
    response = send_and_receive(json.dumps(test_message))
    assert json.loads(response) == test_message  # Parse JSON response


def test_llm_mode():
    """Test LLM mode returns some response"""
    if "ECHO_MODE" in os.environ:
        del os.environ["ECHO_MODE"]

    if not os.getenv("ANTHROPIC_API_KEY"):
        pytest.skip("ANTHROPIC_API_KEY not set")

    request = {"prompt": "Say exactly this: TEST_RESPONSE_MARKER"}

    response = send_and_receive(json.dumps(request))
    assert len(response) > 0
    assert "error" not in response.lower()
