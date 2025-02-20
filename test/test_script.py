import socket
import json


def send_request_and_stream_response(prompt):
    # Connect to the server
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect(("localhost", 8080))
        s.settimeout(30)  # Add timeout

        # Create and send request
        request = {"type": "LLM", "prompt": prompt}
        print(f"Sending request: {request}")
        s.sendall(f"{json.dumps(request)}\n".encode())

        # Stream response
        print("\nReceiving response:")
        print("-" * 50)
        buffer = ""
        try:
            while True:
                chunk = s.recv(1024).decode()
                if not chunk:
                    print("\nConnection closed by server")
                    break
                print(f"Got chunk of length: {len(chunk)}")
                buffer += chunk
                print(chunk, end="", flush=True)
        except socket.timeout:
            print("\nSocket timeout - no more data")
        except Exception as e:
            print(f"\nError receiving data: {e}")

        print("\n" + "-" * 50)
        print(f"Total response length: {len(buffer)}")


if __name__ == "__main__":
    test_prompt = "Write a haiku about rust programming"
    send_request_and_stream_response(test_prompt)
