import socket
import json


def send_request_and_stream_response(prompt):
    # Connect to the server
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect(("localhost", 8080))

        # Create and send request
        request = {"type": "LLM", "prompt": prompt}
        print(f"Sending request: {request}")
        s.sendall(f"{json.dumps(request)}\n".encode())

        # Stream response
        print("\nReceiving response:")
        print("-" * 50)
        while True:
            chunk = s.recv(1024).decode()
            if not chunk:
                break
            print(chunk, end="", flush=True)  # Print chunks as they arrive
        print("\n" + "-" * 50)


if __name__ == "__main__":
    test_prompt = "Write a haiku about rust programming"
    send_request_and_stream_response(test_prompt)
