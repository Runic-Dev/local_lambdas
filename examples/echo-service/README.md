# Echo Service Example

This is a simple example child process that demonstrates the local_lambdas protocol.

## Overview

The echo service:
- Accepts HTTP requests through the local_lambdas proxy
- Returns a JSON response containing information about the received request
- Demonstrates cross-platform named pipe communication

## Running the Example

1. Build local_lambdas:
```bash
cd /home/runner/work/local_lambdas/local_lambdas
cargo build --release
```

2. Start the service with the example manifest:
```bash
./target/release/local_lambdas examples/echo-service/manifest.xml
```

3. In another terminal, send test requests:
```bash
# Simple GET request
curl http://localhost:3000/echo/test

# POST request with data
curl -X POST http://localhost:3000/echo/data \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello, world!"}'

# GET request with query parameters
curl http://localhost:3000/echo/query?foo=bar&baz=qux
```

## Expected Response

The service returns a JSON response like:

```json
{
  "service": "echo-service",
  "message": "Request received successfully",
  "request": {
    "method": "GET",
    "uri": "/echo/test",
    "headers": {
      "host": "localhost:3000",
      "user-agent": "curl/7.x.x"
    },
    "body": ""
  }
}
```

## Implementation Details

The echo service demonstrates:

1. **Reading pipe address**: Gets the `PIPE_ADDRESS` environment variable
2. **Named pipe server**: Creates a server on the specified pipe address
3. **Request handling**: Parses JSON-formatted HTTP requests
4. **Response formatting**: Returns JSON-formatted HTTP responses
5. **Cross-platform support**: Works on both Unix and Windows (with pywin32)

## Requirements

- Python 3.6+
- On Windows: `pywin32` package (`pip install pywin32`)

## Protocol

### Request Format
```json
{
  "method": "GET",
  "uri": "/echo/test",
  "headers": [["Host", "localhost:3000"], ["User-Agent", "curl/7.x.x"]],
  "body": "base64-encoded-body"
}
```

### Response Format
```json
{
  "status": 200,
  "headers": {
    "Content-Type": "application/json",
    "X-Service": "echo-service"
  },
  "body": "base64-encoded-response-body"
}
```
