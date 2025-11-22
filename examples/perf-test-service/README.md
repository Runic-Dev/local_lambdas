# Performance Test Service

This example demonstrates the performance difference between named pipe and HTTP communication modes in local_lambdas.

## Important: Separate Implementations

To accurately measure performance differences, this example includes **two separate service implementations**:

### 1. Pipe-Only Service (`pipe_only_service.py`)
- **NO HTTP server code** - intentionally minimal
- **NO HTTP dependencies** - uses only standard library socket/pipe modules
- **Fast startup** - minimal initialization overhead
- Demonstrates the performance advantage of processes without web server initialization

### 2. HTTP-Only Service (`http_only_service.py`)
- **Includes HTTP server** - uses Python's built-in HTTPServer
- **Web server initialization** - demonstrates realistic startup overhead
- Shows the time cost of initializing a web server layer

## Why Separate Implementations?

The key performance difference isn't just in the communication protocol - it's in the **process startup time**:

- **Pipe-mode processes** can be minimal and start instantly (no HTTP packages to load)
- **HTTP-mode processes** must initialize a web server, which adds startup overhead

This test demonstrates the **total cost** of each approach, including:
1. Process startup time
2. Communication latency
3. Request/response handling

## Running the Performance Test

1. Make scripts executable:
```bash
chmod +x examples/perf-test-service/pipe_only_service.py
chmod +x examples/perf-test-service/http_only_service.py
```

2. Start local_lambdas with the performance test manifest:
```bash
cargo run --release -- examples/perf-test-service/manifest.xml
```

3. In another terminal, run performance tests:

```bash
# Test named pipe communication (minimal service)
time for i in {1..100}; do curl -s http://localhost:3000/pipe/test > /dev/null; done

# Test HTTP communication (service with HTTP server)
time for i in {1..100}; do curl -s http://localhost:3000/http/test > /dev/null; done
```

## Performance Testing

For more accurate benchmarking, use a tool like `hey`:

```bash
# Install hey
go install github.com/rakyll/hey@latest

# Benchmark pipe mode (minimal service)
hey -n 1000 -c 10 http://localhost:3000/pipe/test

# Benchmark HTTP mode (HTTP server overhead)
hey -n 1000 -c 10 http://localhost:3000/http/test
```

## Expected Results

**Named Pipes (pipe_only_service.py)** typically show:
- ✓ **Faster process startup** - no HTTP server initialization
- ✓ **Lower latency per request** - direct IPC
- ✓ **Better performance for sequential requests**
- ✓ **Minimal memory footprint**

**HTTP (http_only_service.py)** typically show:
- ⚠ **Slower process startup** - web server initialization overhead
- ⚠ **Higher latency per request** - HTTP protocol overhead
- ✓ **Better scalability** - can handle concurrent connections
- ⚠ **Higher memory usage** - HTTP server in memory

## Requirements

- Python 3.6+
- On Windows: `pywin32` package (`pip install pywin32`) for pipe_only_service

## Architecture Note

This test accurately reflects real-world scenarios:
- Processes using named pipes can be **minimal microservices** without HTTP frameworks
- Processes using HTTP must include web server code, increasing startup time and memory usage

The performance difference includes **both** communication overhead **and** process initialization overhead.
