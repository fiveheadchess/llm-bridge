import socket
import json
import requests


def stream_response_tcp(prompt):
    # Connect to the server
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect(("localhost", 8080))
        # Create and send request
        request = {"type": "LLM", "prompt": prompt}
        print(f"Sending TCP request: {request}")
        s.sendall(f"{json.dumps(request)}\n".encode())
        # Stream response
        print("\nReceiving TCP response:")
        print("-" * 50)
        while True:
            chunk = s.recv(1024).decode()
            if not chunk:
                break
            print(chunk, end="", flush=True)  # Print chunks as they arrive
        print("\n" + "-" * 50)


def stream_response_rest(prompt):
    # Create the request
    request = {"type": "LLM", "prompt": prompt}
    print(f"Sending REST request: {request}")

    # Make the request with streaming enabled
    url = "http://localhost:8081/llm"
    headers = {"Content-Type": "application/json", "Accept": "text/event-stream"}

    # Use streaming response
    print("\nReceiving REST response:")
    print("-" * 50)

    # Make request and stream the response
    with requests.post(url, json=request, headers=headers, stream=True) as response:
        response.raise_for_status()  # Raise an exception for HTTP errors

        # Iterate through the response chunks as they come
        for chunk in response.iter_content(chunk_size=1024, decode_unicode=True):
            if chunk:
                print(chunk, end="", flush=True)

    print("\n" + "-" * 50)


if __name__ == "__main__":
    test_prompt = "Write a haiku about rust programming"

    print("Testing TCP streaming:")
    stream_response_tcp(test_prompt)

    print("\n\nTesting REST streaming:")
    stream_response_rest(test_prompt)
