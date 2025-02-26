import socket
import json
import requests
import sys
import time


def stream_response_tcp(prompt):
    """Legacy TCP streaming - will only work with the old server implementation"""
    try:
        # Connect to the server
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(5)  # 5 second timeout
            s.connect(("localhost", 8080))

            # Create and send request
            request = {"type": "LLM", "prompt": prompt}
            print(f"Sending TCP request: {request}")
            s.sendall(f"{json.dumps(request)}\n".encode())

            # Stream response
            print("\nReceiving TCP response:")
            print("-" * 50)

            buffer = ""
            while True:
                try:
                    chunk = s.recv(1024).decode()
                    if not chunk:
                        break
                    buffer += chunk
                    print(chunk, end="", flush=True)  # Print chunks as they arrive
                except socket.timeout:
                    print("\nConnection timed out - server may be using HTTP only")
                    break

            print("\n" + "-" * 50)
            print(f"Total response length: {len(buffer)} characters")
    except ConnectionRefusedError:
        print("TCP connection refused - server may be using HTTP only")
    except Exception as e:
        print(f"TCP test error: {e}")


def stream_response_rest(prompt):
    """HTTP REST streaming - works with the new server implementation"""
    try:
        # Create the request
        request = {"prompt": prompt}  # Updated format for the new server
        print(f"Sending REST request: {request}")

        # Make the request with streaming enabled
        url = "http://localhost:8080/api/llm"  # Updated URL path and port
        headers = {"Content-Type": "application/json"}

        # Use streaming response
        print("\nReceiving REST response:")
        print("-" * 50)

        # Make request and stream the response
        start_time = time.time()
        buffer = ""
        chunk_count = 0

        with requests.post(url, json=request, headers=headers, stream=True) as response:
            response.raise_for_status()  # Raise an exception for HTTP errors

            # Debug info
            print(f"Response headers: {response.headers}")
            print(f"Response status: {response.status_code}")

            # Iterate through the response chunks as they come
            for chunk in response.iter_content(chunk_size=None, decode_unicode=True):
                if chunk:
                    chunk_count += 1
                    buffer += chunk
                    print(chunk, end="", flush=True)

                    # Reset timeout on each chunk
                    start_time = time.time()

                # Check for timeout
                if time.time() - start_time > 30:  # 30 second timeout
                    print("\nRequest timed out")
                    break

        print("\n" + "-" * 50)
        print(
            f"Total response length: {len(buffer)} characters in {chunk_count} chunks"
        )
    except requests.exceptions.RequestException as e:
        print(f"REST test error: {e}")


def test_echo_endpoint():
    """Test the echo endpoint"""
    try:
        # Create the request
        message = "Hello from Python test client!"
        request = {"message": message}
        print(f"Sending echo request: {request}")

        # Make the request
        url = "http://localhost:8080/api/echo"
        headers = {"Content-Type": "application/json"}
        response = requests.post(url, json=request, headers=headers)

        # Print the response
        print("\nReceived echo response:")
        print("-" * 50)
        print(response.text)
        print("-" * 50)

        # Verify the response
        try:
            response_json = response.json()
            if response_json.get("message") == message:
                print("Echo test successful!")
            else:
                print("Echo test failed: unexpected response")
        except json.JSONDecodeError:
            print("Echo test failed: response is not valid JSON")

    except requests.exceptions.RequestException as e:
        print(f"Echo test error: {e}")


if __name__ == "__main__":
    test_prompt = "Write a haiku about rust programming"

    if len(sys.argv) > 1 and sys.argv[1] == "--tcp-only":
        print("Testing TCP streaming only:")
        stream_response_tcp(test_prompt)
    elif len(sys.argv) > 1 and sys.argv[1] == "--rest-only":
        print("Testing REST streaming only:")
        stream_response_rest(test_prompt)
    elif len(sys.argv) > 1 and sys.argv[1] == "--echo-only":
        print("Testing echo endpoint only:")
        test_echo_endpoint()
    else:
        print("Testing REST streaming:")
        stream_response_rest(test_prompt)
