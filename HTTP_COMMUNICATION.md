# HTTP Communication Mode - Implementation Summary

## Overview

This implementation adds HTTP as an alternative communication mode to named pipes in local_lambdas, providing flexibility for different use cases and process types.

## Architecture Changes

### 1. Domain Layer
- **New**: `CommunicationMode` enum (Pipe, Http)
- **Updated**: `Process` entity includes `communication_mode` field
- **New**: Utility functions for address generation (`domain/utils.rs`)
  - `get_pipe_address_from_name()` - Generate pipe address
  - `get_http_address_from_name()` - Generate HTTP address
  - `get_http_port_from_name()` - Deterministic port from name (9000-9999)

### 2. Configuration
- **Updated**: `ProcessConfig` with optional `communication_mode` field
- Defaults to "pipe" for backward compatibility
- Accepts "pipe" or "http" values

### 3. Infrastructure Layer
- **New**: `HttpClient` implementing `PipeCommunicationService`
- Uses `reqwest` for HTTP communication
- Same interface as `NamedPipeClient` for consistency

### 4. Adapters
- **Updated**: `TokioProcessOrchestrator` sets environment variables:
  - `PIPE_ADDRESS` for pipe mode
  - `HTTP_ADDRESS` for HTTP mode
- **Updated**: `XmlProcessRepository` parses communication mode from config

### 5. Use Cases
- **Updated**: `ProxyHttpRequestUseCase` routes based on communication mode
- Automatically selects correct address for each process

## Key Design Decisions

### 1. Deterministic Port Generation
- HTTP mode processes get ports in range 9000-9999
- Port generated from `pipe_name` hash for consistency
- Same process always gets same port across restarts

### 2. Separate Test Services
- **Critical**: Pipe-only service has NO HTTP code
- Demonstrates real startup time differences
- HTTP-only service includes web server initialization

### 3. Environment Variables
- Processes discover communication mode via environment:
  - `PIPE_ADDRESS`: Pipe mode
  - `HTTP_ADDRESS`: HTTP mode
- Allows same protocol implementation in child processes

## Testing

### Unit Tests
- 43 library tests covering domain, adapters, and use cases
- Tests for address generation utilities
- Tests for communication mode parsing

### Integration Tests
- 7 tests for manifest loading and orchestration
- Tests for both pipe and HTTP modes

### E2E Tests
- 4 performance comparison tests
- Separate implementations for accurate benchmarking
- Demonstrates startup time and latency differences

### Performance Test (`cargo test --test perf_comparison_tests -- --ignored`)
```bash
=== Performance Comparison: Named Pipes vs HTTP ===
NOTE: This test compares:
  • Pipe service: Minimal Python script (NO HTTP server code)
  • HTTP service: Python with HTTPServer (web server overhead)

Testing PIPE mode (50 requests)...
  → Using minimal service with NO HTTP dependencies
  Total time: ~XXXms
  Average per request: ~XXms
  
Testing HTTP mode (50 requests)...
  → Using service with HTTP server initialization overhead
  Total time: ~XXXms
  Average per request: ~XXms
```

## Examples

### Performance Test Services
Located in `examples/perf-test-service/`:
- `pipe_only_service.py` - Minimal service (no HTTP imports)
- `http_only_service.py` - HTTP server-based service
- `manifest.xml` - Configuration for both modes
- `README.md` - Detailed benchmarking instructions

## Usage Example

```xml
<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <!-- Fast startup, minimal process -->
    <process>
        <id>minimal-service</id>
        <executable>./bin/minimal</executable>
        <route>/api/*</route>
        <pipe_name>minimal_pipe</pipe_name>
        <communication_mode>pipe</communication_mode>
    </process>
    
    <!-- Process with HTTP framework -->
    <process>
        <id>web-service</id>
        <executable>dotnet</executable>
        <arg>WebService.dll</arg>
        <route>/web/*</route>
        <pipe_name>web_http</pipe_name>
        <communication_mode>http</communication_mode>
    </process>
</manifest>
```

## Benefits

### Named Pipe Mode
- ✅ Fast process startup
- ✅ Low latency
- ✅ Minimal memory footprint
- ✅ Best for simple, focused microservices
- ✅ No HTTP framework needed

### HTTP Mode
- ✅ Use existing HTTP frameworks (Kestrel, Flask, Express)
- ✅ Better concurrency handling
- ✅ Familiar programming model
- ✅ Good for services that need HTTP anyway
- ⚠️ Slower startup and higher memory usage

## Future Enhancements

1. **Request Caching** - Cache responses to reduce process communication
2. **Idle Process Shutdown** - Stop processes after N cached requests
3. **Auto-restart** - Restart stopped processes on cache miss
4. **.NET Examples** - Demonstrate with C#/Kestrel processes
5. **Connection Pooling** - Reuse HTTP connections for better performance

## Security Considerations

- Both modes use localhost-only communication
- HTTP mode uses deterministic ports (not configurable by child)
- All communication is local IPC (no external exposure)
- Same JSON protocol for both modes ensures consistent validation

## Compatibility

- ✅ Backward compatible - existing configs work unchanged
- ✅ Default to pipe mode if not specified
- ✅ Can mix modes in same manifest
- ✅ Clean Architecture ensures easy future extensions
