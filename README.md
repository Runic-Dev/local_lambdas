# local_lambdas

A Rust-based HTTP proxy that orchestrates local processes and communicates with them via named pipes or HTTP. This application allows you to configure and manage multiple executables that can handle HTTP requests through a centralized proxy server.

## Features

- **Flexible communication modes** - Support for both named pipes and HTTP communication
- **Cross-platform named pipe communication** - Works on both Windows and Unix-like systems
- **HTTP-based communication** - Option to use HTTP for inter-process communication
- **XML-based configuration** - Easy-to-edit manifest.xml file for process management
- **HTTP proxy server** - Routes HTTP requests to the appropriate process based on URL patterns
- **Process orchestration** - Automatically starts and manages child processes
- **Graceful shutdown** - Handles Ctrl+C and properly cleans up child processes

## Architecture

The application consists of four main components:

1. **Config Module** - Parses the manifest.xml configuration file
2. **Communication Layer** - Handles both named pipe and HTTP communication
3. **Process Orchestrator** - Manages the lifecycle of child processes
4. **HTTP Proxy** - Routes incoming HTTP requests to the appropriate process

## Configuration

Create a `manifest.xml` file with your process configurations:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <!-- Process using named pipes (default) -->
    <process>
        <id>api-service</id>
        <executable>./bin/api-service.exe</executable>
        <arg>--port</arg>
        <arg>8080</arg>
        <route>/api/*</route>
        <pipe_name>api_service_pipe</pipe_name>
        <working_dir>./services/api</working_dir>
        <communication_mode>pipe</communication_mode>
    </process>
    
    <!-- Process using HTTP communication -->
    <process>
        <id>auth-service</id>
        <executable>./bin/auth-service.exe</executable>
        <route>/auth/*</route>
        <pipe_name>auth_service_http</pipe_name>
        <communication_mode>http</communication_mode>
    </process>
</manifest>
```

### Configuration Elements

- **id**: Unique identifier for the process
- **executable**: Path to the executable file
- **arg**: Command-line argument (can have multiple)
- **route**: HTTP URL pattern to match (supports wildcards with `/*`)
- **pipe_name**: Name identifier for communication (used for pipe name or HTTP port generation)
- **working_dir**: (Optional) Working directory for the process
- **communication_mode**: (Optional) Communication mode - `pipe` (default) or `http`

## Usage

### Building

```bash
cargo build --release
```

### Running

```bash
# Use default manifest.xml in current directory
./target/release/local_lambdas

# Specify custom manifest file
./target/release/local_lambdas path/to/custom-manifest.xml

# Set custom bind address (default: 127.0.0.1:3000)
BIND_ADDRESS=0.0.0.0:8080 ./target/release/local_lambdas
```

### Environment Variables

- **BIND_ADDRESS**: HTTP server bind address (default: `127.0.0.1:3000`)
- **RUST_LOG**: Logging level (e.g., `debug`, `info`, `warn`, `error`)

## Child Process Protocol

Child processes can communicate using either **named pipes** or **HTTP**, depending on the `communication_mode` configuration.

### Named Pipe Mode (Default)

Child processes receive the pipe address through the `PIPE_ADDRESS` environment variable and must:

1. **Connect to the named pipe** when ready to handle requests
2. **Read HTTP request data** in JSON format:
```json
{
    "method": "GET",
    "uri": "/api/example",
    "headers": [["Content-Type", "application/json"]],
    "body": "base64-encoded-body"
}
```
3. **Write HTTP response data** in JSON format:
```json
{
    "status": 200,
    "headers": {
        "Content-Type": "application/json"
    },
    "body": "base64-encoded-response"
}
```

**Named Pipe Addresses:**
- **Windows**: `\\.\pipe\{pipe_name}`
- **Unix/Linux/macOS**: `/tmp/{pipe_name}`

### HTTP Mode

Child processes receive the HTTP address through the `HTTP_ADDRESS` environment variable (e.g., `127.0.0.1:9123`) and must:

1. **Start an HTTP server** on the provided address
2. **Accept POST requests** with the same JSON format as pipe mode
3. **Return responses** with the same JSON format as pipe mode

**Benefits:**
- No need to implement pipe handling
- Can use standard HTTP frameworks (Kestrel, Flask, Express, etc.)
- Better for processes that already have an HTTP server

**Trade-offs:**
- Slower process startup (HTTP server initialization)
- Higher memory usage
- Slightly higher latency per request

## Communication Mode Comparison

| Aspect | Named Pipes | HTTP |
|--------|-------------|------|
| **Process startup** | ⚡ Fast (no HTTP server) | 🐌 Slower (HTTP server init) |
| **Latency** | 🚀 Lower (direct IPC) | 📡 Higher (HTTP overhead) |
| **Memory** | 💾 Minimal | 💾 Higher (HTTP stack) |
| **Concurrency** | ⚠️ Single connection | ✅ Multiple connections |
| **Use case** | Minimal microservices | Services with web frameworks |

## Example Child Process

Here's a simple Python example that implements the protocol:

```python
import os
import json
import base64
import socket

pipe_address = os.environ['PIPE_ADDRESS']

# Connect to named pipe (Unix example)
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect(pipe_address)

# Read request
request_data = sock.recv(4096)
request = json.loads(request_data)

# Process request
method = request['method']
uri = request['uri']
body = base64.b64decode(request['body'])

# Create response
response = {
    'status': 200,
    'headers': {'Content-Type': 'text/plain'},
    'body': base64.b64encode(b'Hello from child process!').decode()
}

# Send response
sock.sendall(json.dumps(response).encode())
sock.close()
```

## Development

### Running Tests

```bash
cargo test
```

### Logging

Enable debug logging:

```bash
RUST_LOG=debug ./target/release/local_lambdas
```

## Performance

For detailed performance benchmarks comparing HTTP vs named pipe communication, see:

- **[PERFORMANCE_METRICS.md](PERFORMANCE_METRICS.md)** - Case study with actual test results and metrics
- **[PERFORMANCE_TEST_DOCUMENTATION.md](PERFORMANCE_TEST_DOCUMENTATION.md)** - Architecture and methodology documentation

### Quick Performance Summary

| Communication Mode | Avg Response Time | Throughput |
|-------------------|-------------------|------------|
| Cached Response | 0.42ms | 2,380 req/s |
| HTTP (Warm) | 8.75ms | 114 req/s |
| Named Pipe (Warm) | 3.21ms | 311 req/s |

Named pipes provide **63% faster response times** and **2.7x higher throughput** compared to HTTP for minimal services.

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.