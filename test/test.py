import pytest
import asyncio
import websockets
import json
import os
from websockets.exceptions import ConnectionClosed

BASE_URL = "ws://localhost:8080/ws"


@pytest.mark.asyncio
async def test_stream_response():
    """Test for successful streaming response"""
    request_data = {
        "model": "claude-3-sonnet-20240229",
        "max_tokens": 1000,
        "messages": [
            {
                "role": "user",
                "content": "This is an automated test. Please respond with exactly these words: 'TEST_MARKER: Automated test response complete' and nothing else.",
            }
        ],
        "stream": True,
    }

    try:
        async with websockets.connect(BASE_URL) as websocket:
            await websocket.send(json.dumps(request_data))

            full_response = ""
            while True:
                try:
                    response = await asyncio.wait_for(websocket.recv(), timeout=5.0)
                    full_response += response

                    if "TEST_MARKER: Automated test response complete" in full_response:
                        break
                    if "error" in full_response.lower():
                        pytest.fail(f"Received error in response: {full_response}")
                except asyncio.TimeoutError:
                    pytest.fail("Timeout waiting for response")

            assert "TEST_MARKER: Automated test response complete" in full_response

    except ConnectionClosed as e:
        pytest.fail(f"WebSocket connection closed unexpectedly: {e}")
    except Exception as e:
        pytest.fail(f"Unexpected error: {e}")


@pytest.mark.asyncio
async def test_invalid_request():
    """Test for handling invalid requests"""
    request_data = {
        "model": "claude-3-sonnet-20240229",
        "max_tokens": 1000,
        "messages": [],  # Empty messages should trigger an error
        "stream": True,
    }

    try:
        async with websockets.connect(BASE_URL) as websocket:
            await websocket.send(json.dumps(request_data))

            try:
                response = await asyncio.wait_for(websocket.recv(), timeout=5.0)
                response_data = (
                    response if isinstance(response, str) else response.decode()
                )

                # The Claude API should return an error for empty messages
                assert (
                    "error" in response_data.lower()
                    or "invalid" in response_data.lower()
                ), f"Expected error response for invalid request, got: {response_data}"

            except asyncio.TimeoutError:
                pytest.fail("Timeout waiting for error response")

    except ConnectionClosed as e:
        # Connection closure might be an expected behavior for invalid requests
        assert (
            str(e).lower().find("error") != -1 or str(e).lower().find("invalid") != -1
        ), f"Unexpected connection closure: {e}"
    except Exception as e:
        pytest.fail(f"Unexpected error: {e}")


@pytest.mark.asyncio
async def test_connection_stability():
    """Test WebSocket connection stability"""
    request_data = {
        "model": "claude-3-sonnet-20240229",
        "max_tokens": 1000,
        "messages": [{"role": "user", "content": "Send a short response"}],
        "stream": True,
    }

    try:
        async with websockets.connect(BASE_URL) as websocket:
            # Test ping/pong
            await websocket.ping()

            # Send request
            await websocket.send(json.dumps(request_data))

            # Verify we can receive multiple messages
            message_count = 0
            timeout = 5.0

            while timeout > 0:
                try:
                    start_time = asyncio.get_event_loop().time()
                    response = await asyncio.wait_for(websocket.recv(), timeout=timeout)
                    message_count += 1

                    if "stop reason" in response.lower():
                        break

                    timeout -= asyncio.get_event_loop().time() - start_time
                except asyncio.TimeoutError:
                    break

            assert message_count > 0, "Should receive at least one message"

    except Exception as e:
        pytest.fail(f"Connection stability test failed: {e}")


@pytest.fixture(scope="session")
def event_loop():
    """Create an instance of the default event loop for each test case."""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()
