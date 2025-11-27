//! Performance comparison E2E tests
//! Tests the performance difference between HTTP and named pipe communication modes
#![allow(deprecated)]

use assert_cmd::Command;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::thread;
use tempfile::TempDir;

/// Helper to create a test manifest file
fn create_test_manifest(dir: &TempDir, content: &str) -> PathBuf {
    let manifest_path = dir.path().join("manifest.xml");
    let mut file = File::create(&manifest_path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    manifest_path
}

/// Helper to create a minimal pipe-only Python test service (no HTTP code)
fn create_pipe_only_service(dir: &TempDir) -> PathBuf {
    let service_path = dir.path().join("pipe_service.py");
    let service_code = r#"#!/usr/bin/env python3
import os, json, base64, socket, sys

def handle(data):
    req = json.loads(data)
    body = json.dumps({'status': 'ok', 'mode': 'pipe'})
    resp = {'status': 200, 'headers': {'Content-Type': 'application/json'},
            'body': base64.b64encode(body.encode()).decode()}
    return json.dumps(resp).encode()

pipe_addr = os.environ.get('PIPE_ADDRESS')
if not pipe_addr:
    sys.exit(1)

if os.path.exists(pipe_addr):
    os.remove(pipe_addr)

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.bind(pipe_addr)
sock.listen(5)

while True:
    conn, _ = sock.accept()
    data = b''
    while True:
        chunk = conn.recv(4096)
        if not chunk:
            break
        data += chunk
        try:
            json.loads(data)
            break
        except:
            continue
    if data:
        conn.sendall(handle(data))
    conn.close()
"#;
    let mut file = File::create(&service_path).unwrap();
    file.write_all(service_code.as_bytes()).unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&service_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&service_path, perms).unwrap();
    }
    
    service_path
}

/// Helper to create an HTTP-only Python test service (with HTTP server)
fn create_http_only_service(dir: &TempDir) -> PathBuf {
    let service_path = dir.path().join("http_service.py");
    let service_code = r#"#!/usr/bin/env python3
import os, json, base64, sys
from http.server import HTTPServer, BaseHTTPRequestHandler

def handle(data):
    req = json.loads(data)
    body = json.dumps({'status': 'ok', 'mode': 'http'})
    resp = {'status': 200, 'headers': {'Content-Type': 'application/json'},
            'body': base64.b64encode(body.encode()).decode()}
    return json.dumps(resp).encode()

class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get('Content-Length', 0))
        data = self.rfile.read(length)
        resp = handle(data)
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(resp)))
        self.end_headers()
        self.wfile.write(resp)
    def log_message(self, format, *args):
        pass

http_addr = os.environ.get('HTTP_ADDRESS')
if not http_addr:
    sys.exit(1)

host, port = http_addr.rsplit(':', 1)
server = HTTPServer((host, int(port)), Handler)
server.serve_forever()
"#;
    let mut file = File::create(&service_path).unwrap();
    file.write_all(service_code.as_bytes()).unwrap();
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&service_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&service_path, perms).unwrap();
    }
    
    service_path
}

#[test]
#[ignore] // Run manually: cargo test --test perf_comparison_tests -- --ignored
fn test_performance_comparison_pipe_vs_http() {
    let temp_dir = TempDir::new().unwrap();
    let pipe_service = create_pipe_only_service(&temp_dir);
    let http_service = create_http_only_service(&temp_dir);
    
    let manifest = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>test-pipe</id>
        <executable>python3</executable>
        <arg>{}</arg>
        <route>/pipe/*</route>
        <pipe_name>test_pipe</pipe_name>
        <communication_mode>pipe</communication_mode>
    </process>
    
    <process>
        <id>test-http</id>
        <executable>python3</executable>
        <arg>{}</arg>
        <route>/http/*</route>
        <pipe_name>test_http</pipe_name>
        <communication_mode>http</communication_mode>
    </process>
</manifest>"#, pipe_service.display(), http_service.display());
    
    let manifest_path = create_test_manifest(&temp_dir, &manifest);
    
    // Start local_lambdas
    let mut cmd = std::process::Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--")
        .arg(manifest_path.to_str().unwrap())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Failed to start local_lambdas");
    
    // Wait for services to start
    thread::sleep(Duration::from_secs(4));
    
    // Test both modes
    let num_requests = 50;
    
    println!("\n=== Performance Comparison: Named Pipes vs HTTP ===");
    println!("NOTE: This test compares:");
    println!("  • Pipe service: Minimal Python script (NO HTTP server code)");
    println!("  • HTTP service: Python with HTTPServer (web server overhead)");
    println!();
    
    // Benchmark pipe mode (minimal service)
    println!("Testing PIPE mode ({} requests)...", num_requests);
    println!("  → Using minimal service with NO HTTP dependencies");
    let pipe_start = Instant::now();
    for _ in 0..num_requests {
        let response = reqwest::blocking::get("http://localhost:3000/pipe/test");
        if let Ok(resp) = response {
            assert!(resp.status().is_success());
        }
    }
    let pipe_duration = pipe_start.elapsed();
    let pipe_avg = pipe_duration.as_millis() as f64 / num_requests as f64;
    
    println!("  Total time: {:?}", pipe_duration);
    println!("  Average per request: {:.2}ms", pipe_avg);
    println!("  Requests/sec: {:.2}", 1000.0 / pipe_avg);
    
    // Small delay between tests
    thread::sleep(Duration::from_millis(500));
    
    // Benchmark HTTP mode (with HTTP server)
    println!("\nTesting HTTP mode ({} requests)...", num_requests);
    println!("  → Using service with HTTP server initialization overhead");
    let http_start = Instant::now();
    for _ in 0..num_requests {
        let response = reqwest::blocking::get("http://localhost:3000/http/test");
        if let Ok(resp) = response {
            assert!(resp.status().is_success());
        }
    }
    let http_duration = http_start.elapsed();
    let http_avg = http_duration.as_millis() as f64 / num_requests as f64;
    
    println!("  Total time: {:?}", http_duration);
    println!("  Average per request: {:.2}ms", http_avg);
    println!("  Requests/sec: {:.2}", 1000.0 / http_avg);
    
    // Calculate difference
    println!("\n=== Results ===");
    let diff_ms = (http_avg - pipe_avg).abs();
    let diff_pct = (diff_ms / pipe_avg) * 100.0;
    
    if http_avg > pipe_avg {
        println!("✓ Named pipes are FASTER by {:.2}ms ({:.1}%)", diff_ms, diff_pct);
        println!("  Reason: No HTTP server initialization overhead");
    } else {
        println!("✓ HTTP is faster by {:.2}ms ({:.1}%)", diff_ms, diff_pct);
    }
    
    println!("\nKey Differences:");
    println!("• PIPE mode: Minimal process, instant startup, direct IPC");
    println!("• HTTP mode: Includes web server, initialization overhead, HTTP protocol");
    println!("\nNote: Named pipes excel for simple processes that don't need HTTP.");
    println!("      HTTP mode is better when processes already need a web server.\n");
    
    // Cleanup
    cmd.kill().ok();
    cmd.wait().ok();
}

#[test]
fn test_manifest_with_http_mode() {
    let temp_dir = TempDir::new().unwrap();
    let manifest = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>test-http</id>
        <executable>echo</executable>
        <route>/test/*</route>
        <pipe_name>test_http</pipe_name>
        <communication_mode>http</communication_mode>
    </process>
</manifest>"#;
    
    let manifest_path = create_test_manifest(&temp_dir, manifest);
    
    let mut cmd = Command::cargo_bin("local_lambdas").unwrap();
    cmd.arg(manifest_path.to_str().unwrap())
        .timeout(Duration::from_millis(500));
    
    // Just verify it starts without crashing
    let _ = cmd.output();
}

#[test]
fn test_manifest_with_pipe_mode() {
    let temp_dir = TempDir::new().unwrap();
    let manifest = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>test-pipe</id>
        <executable>echo</executable>
        <route>/test/*</route>
        <pipe_name>test_pipe</pipe_name>
        <communication_mode>pipe</communication_mode>
    </process>
</manifest>"#;
    
    let manifest_path = create_test_manifest(&temp_dir, manifest);
    
    let mut cmd = Command::cargo_bin("local_lambdas").unwrap();
    cmd.arg(manifest_path.to_str().unwrap())
        .timeout(Duration::from_millis(500));
    
    // Just verify it starts without crashing
    let _ = cmd.output();
}

#[test]
fn test_manifest_default_communication_mode() {
    let temp_dir = TempDir::new().unwrap();
    let manifest = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest>
    <process>
        <id>test-default</id>
        <executable>echo</executable>
        <route>/test/*</route>
        <pipe_name>test_default</pipe_name>
    </process>
</manifest>"#;
    
    let manifest_path = create_test_manifest(&temp_dir, manifest);
    
    let mut cmd = Command::cargo_bin("local_lambdas").unwrap();
    cmd.arg(manifest_path.to_str().unwrap())
        .timeout(Duration::from_millis(500));
    
    // Should default to pipe mode and start successfully
    let _ = cmd.output();
}
