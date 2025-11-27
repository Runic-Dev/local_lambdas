# .NET Performance Tests for local_lambdas

This directory contains comprehensive performance tests for the local_lambdas proxy using .NET services.

## Overview

The performance tests measure 5 different scenarios as required:

1. **Case 1**: Cached response (request with cached response)
2. **Case 2**: HTTP forward to running process (.NET process already started)
3. **Case 3**: HTTP forward with cold start (.NET process not running, needs to start)
4. **Case 4**: Named pipe forward to running process (.NET process already started)
5. **Case 5**: Named pipe forward with cold start (.NET process not running, needs to start)

## Key Design Decisions

### Separate Services for HTTP and Named Pipes

This implementation follows a critical requirement:

- **HttpService**: Contains ASP.NET Core dependencies and runs an HTTP server
- **PipeService**: Contains NO HTTP dependencies - only uses System.IO.Pipes and System.Net.Sockets

This separation is essential because:
1. Named pipe services can be minimal with fast startup
2. HTTP services require web server initialization overhead
3. This accurately reflects real-world performance differences

### Verification of No HTTP Dependencies

You can verify that PipeService has no HTTP dependencies:

```bash
cd PipeService
dotnet list package
# Output: No packages were found for this framework.
```

Compare with HttpService which includes ASP.NET Core packages.

## Project Structure

```
dotnet-perf-test/
├── HttpService/          # .NET service with HTTP server (has HTTP dependencies)
│   ├── HttpService.csproj
│   └── Program.cs
├── PipeService/          # .NET service for named pipes (NO HTTP dependencies)
│   ├── PipeService.csproj
│   └── Program.cs
├── manifest.xml          # Configuration for local_lambdas
├── run_performance_tests.py  # Main performance test script
├── PERFORMANCE_RESULTS.md    # Generated detailed results (after running tests)
├── performance_results.json  # Generated raw JSON data (after running tests)
└── README.md            # This file
```

## Running the Tests

### Prerequisites

- .NET 8.0 SDK
- Rust (cargo)
- Python 3.6+
- requests library: `pip install requests`
- psutil library (optional, for memory/CPU metrics): `pip install psutil`

### Build Services

```bash
# Build HTTP service
cd HttpService
dotnet build

# Build Pipe service  
cd ../PipeService
dotnet build
cd ..
```

### Run Performance Tests

```bash
# From the examples/dotnet-perf-test directory
./run_performance_tests.py
```

Or from the repository root:

```bash
./examples/dotnet-perf-test/run_performance_tests.py
```

The test will:
1. Collect system information (platform, CPU, memory)
2. Build local_lambdas in release mode
3. Run each test case with 50 requests (plus warmup)
4. Measure detailed metrics per test case
5. Save raw results to `performance_results.json`
6. Generate a comprehensive markdown report: `PERFORMANCE_RESULTS.md`

## Metrics Collected

### Response Time Metrics
- **Average**: Mean response time across all requests
- **Standard Deviation**: Measure of response time variability
- **Variance**: Square of standard deviation
- **Min/Max**: Fastest and slowest response times
- **Range**: Difference between max and min

### Percentile Distribution
- **P10, P25, P50 (median), P75, P90, P95, P99, P99.9**
- Complete latency distribution for tail latency analysis

### Memory Metrics (requires psutil)
- **RSS (Resident Set Size)**: Physical memory used
- **VMS (Virtual Memory Size)**: Total virtual memory
- **Memory Growth**: Change during test execution
- **Child Processes**: Number of spawned processes

### CPU Metrics (requires psutil)
- **Average CPU**: Mean utilization during test
- **Max CPU**: Peak utilization
- **Min CPU**: Minimum utilization

### Throughput Metrics
- **Requests per Second**: Overall throughput
- **Response Size**: Average payload size
- **Data Throughput**: Bytes per second

### Distribution Histogram
- ASCII histogram of response time distribution
- 10 bins covering the range from min to max response time

## Expected Results

### Named Pipes (Cases 4 & 5)
- **Faster cold start**: No HTTP server initialization
- **Lower latency**: Direct IPC, no HTTP protocol overhead
- **Minimal memory**: No web server in process memory

### HTTP Communication (Cases 2 & 3)
- **Slower cold start**: HTTP server (Kestrel) initialization required
- **Higher latency**: HTTP protocol overhead
- **Higher memory**: Web server components loaded

### Cached Responses (Case 1)
- **Lowest latency**: No process communication at all
- **Highest throughput**: Response served directly from memory

## Generated Report

The `PERFORMANCE_RESULTS.md` report includes:

### Summary Tables
- Response time comparison across all test cases
- Memory usage comparison (if psutil available)
- CPU usage comparison (if psutil available)

### Detailed Per-Test Analysis
- Request metrics (success rate, status codes)
- Response time statistics
- Percentile distribution table
- Throughput metrics
- Response time histogram (ASCII visualization)
- Memory and CPU metrics (if available)

### Comparative Analysis
- HTTP vs Named Pipe (warm process) comparison
- Cold start performance analysis
- Cache effectiveness analysis
- Percentage improvements with calculations

### Conclusions
- Summary of key findings
- Actionable recommendations
- Technical details and methodology

## Understanding the Results

The performance difference between HTTP and named pipes includes:

1. **Process Startup Time**
   - Pipe service: Instant (minimal .NET runtime)
   - HTTP service: Slower (Kestrel web server initialization)

2. **Per-Request Overhead**
   - Named pipes: Direct IPC, binary protocol
   - HTTP: TCP/IP stack, HTTP protocol parsing

3. **Memory Usage**
   - Pipe service: ~30-50MB (minimal runtime)
   - HTTP service: ~60-100MB (includes web server)

## Implementation Details

### HttpService (Program.cs)

Uses ASP.NET Core with Kestrel:
- WebApplication.CreateBuilder()
- Kestrel server configuration
- HTTP middleware pipeline
- Dependency: Microsoft.AspNetCore.App

### PipeService (Program.cs)

Uses only standard .NET libraries:
- System.IO.Pipes.NamedPipeServerStream (Windows)
- System.Net.Sockets.Socket with UnixDomainSocketEndPoint (Linux/macOS)
- No external packages
- No HTTP server code

## Notes

- Tests run in release mode for accurate performance metrics
- Each test includes warmup requests to ensure JIT compilation
- Cold start times are measured separately from throughput tests
- All .NET processes use the same .NET 8.0 runtime
- Install psutil for memory and CPU metrics collection

## Troubleshooting

### Build Errors

```bash
# Clean and rebuild
cd HttpService && dotnet clean && dotnet build
cd ../PipeService && dotnet clean && dotnet build
```

### Port Already in Use

If port 3000 is in use, you can modify the test script or kill the process:

```bash
lsof -ti:3000 | xargs kill -9
```

### Services Not Starting

Check that the manifest paths are correct and .NET is installed:

```bash
dotnet --version  # Should show 8.0 or higher
```

### Missing Memory/CPU Metrics

Install psutil for complete metrics:

```bash
pip install psutil
```
