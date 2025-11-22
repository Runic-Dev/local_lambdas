#!/usr/bin/env python3
"""
Performance test service that supports both HTTP and named pipe communication.
This service is used to benchmark the performance difference between the two modes.
"""

import os
import json
import base64
import socket
import sys
import logging
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path
import threading

# Configure logging
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('perf-test-service')


def handle_request(request_data):
    """Process an HTTP request and return a response."""
    try:
        request = json.loads(request_data)
        
        method = request.get('method', 'UNKNOWN')
        uri = request.get('uri', '/')
        headers = request.get('headers', [])
        body_b64 = request.get('body', '')
        
        # Decode request body
        if body_b64:
            body = base64.b64decode(body_b64).decode('utf-8', errors='ignore')
        else:
            body = ''
        
        logger.debug(f"Received {method} request for {uri}")
        
        # Create simple response
        response_body = {
            'service': 'perf-test-service',
            'timestamp': time.time(),
            'request': {
                'method': method,
                'uri': uri,
            }
        }
        
        response_json = json.dumps(response_body)
        response_b64 = base64.b64encode(response_json.encode()).decode()
        
        # Create HTTP response
        response = {
            'status': 200,
            'headers': {
                'Content-Type': 'application/json',
            },
            'body': response_b64
        }
        
        return json.dumps(response).encode()
        
    except Exception as e:
        logger.error(f"Error processing request: {e}", exc_info=True)
        
        # Return error response
        error_body = json.dumps({
            'error': str(e),
            'service': 'perf-test-service'
        })
        error_response = {
            'status': 500,
            'headers': {'Content-Type': 'application/json'},
            'body': base64.b64encode(error_body.encode()).decode()
        }
        return json.dumps(error_response).encode()


class HTTPRequestHandler(BaseHTTPRequestHandler):
    """HTTP request handler for HTTP mode."""
    
    def do_POST(self):
        """Handle POST requests."""
        try:
            # Read request body
            content_length = int(self.headers.get('Content-Length', 0))
            request_data = self.rfile.read(content_length)
            
            logger.debug(f"HTTP: Received POST request at {self.path}")
            
            # Handle request
            response_data = handle_request(request_data)
            
            # Send response
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(response_data)))
            self.end_headers()
            self.wfile.write(response_data)
            
        except Exception as e:
            logger.error(f"Error handling HTTP request: {e}", exc_info=True)
            self.send_error(500, str(e))
    
    def log_message(self, format, *args):
        """Suppress default HTTP server logs."""
        pass


def serve_http(address):
    """Serve requests via HTTP."""
    host, port_str = address.rsplit(':', 1)
    port = int(port_str)
    
    logger.info(f"Starting HTTP server on {host}:{port}")
    
    server = HTTPServer((host, port), HTTPRequestHandler)
    
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        logger.info("Received interrupt, shutting down HTTP server...")
    finally:
        server.server_close()


def serve_unix_socket(pipe_address):
    """Serve requests on a Unix domain socket."""
    # Remove existing socket file if it exists
    if os.path.exists(pipe_address):
        os.remove(pipe_address)
    
    # Create Unix socket
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.bind(pipe_address)
    sock.listen(5)
    
    logger.info(f"Listening on Unix socket: {pipe_address}")
    
    try:
        while True:
            conn, _ = sock.accept()
            logger.debug("Accepted new connection")
            
            try:
                # Read request data
                data = b''
                while True:
                    chunk = conn.recv(4096)
                    if not chunk:
                        break
                    data += chunk
                    # Try to parse JSON to see if we have a complete message
                    try:
                        json.loads(data)
                        break
                    except json.JSONDecodeError:
                        continue
                
                if data:
                    # Handle request
                    response = handle_request(data)
                    conn.sendall(response)
                    logger.debug("Sent response")
                    
            except Exception as e:
                logger.error(f"Error handling connection: {e}", exc_info=True)
            finally:
                conn.close()
                
    except KeyboardInterrupt:
        logger.info("Received interrupt, shutting down...")
    finally:
        sock.close()
        if os.path.exists(pipe_address):
            os.remove(pipe_address)


def serve_windows_pipe(pipe_address):
    """Serve requests on a Windows named pipe."""
    import win32pipe
    import win32file
    import pywintypes
    
    logger.info(f"Listening on Windows named pipe: {pipe_address}")
    
    try:
        while True:
            # Create named pipe
            pipe = win32pipe.CreateNamedPipe(
                pipe_address,
                win32pipe.PIPE_ACCESS_DUPLEX,
                win32pipe.PIPE_TYPE_MESSAGE | win32pipe.PIPE_READMODE_MESSAGE | win32pipe.PIPE_WAIT,
                win32pipe.PIPE_UNLIMITED_INSTANCES,
                65536,
                65536,
                0,
                None
            )
            
            logger.debug("Waiting for client connection...")
            win32pipe.ConnectNamedPipe(pipe, None)
            logger.debug("Client connected")
            
            try:
                # Read request
                data = b''
                while True:
                    try:
                        result, chunk = win32file.ReadFile(pipe, 4096)
                        data += chunk
                        if result == 0:
                            break
                    except pywintypes.error as e:
                        if e.args[0] == 109:  # ERROR_BROKEN_PIPE
                            break
                        raise
                
                if data:
                    # Handle request
                    response = handle_request(data)
                    win32file.WriteFile(pipe, response)
                    logger.debug("Sent response")
                    
            except Exception as e:
                logger.error(f"Error handling connection: {e}", exc_info=True)
            finally:
                win32file.CloseHandle(pipe)
                
    except KeyboardInterrupt:
        logger.info("Received interrupt, shutting down...")


def main():
    """Main entry point."""
    # Check for HTTP_ADDRESS first (HTTP mode)
    http_address = os.environ.get('HTTP_ADDRESS')
    pipe_address = os.environ.get('PIPE_ADDRESS')
    
    if not http_address and not pipe_address:
        logger.error("Neither HTTP_ADDRESS nor PIPE_ADDRESS environment variable is set")
        logger.error("This service should be started by local_lambdas")
        sys.exit(1)
    
    logger.info(f"Performance test service starting...")
    
    if http_address:
        logger.info(f"Running in HTTP mode: {http_address}")
        serve_http(http_address)
    else:
        logger.info(f"Running in PIPE mode: {pipe_address}")
        # Determine platform and serve accordingly
        if sys.platform == 'win32':
            serve_windows_pipe(pipe_address)
        else:
            serve_unix_socket(pipe_address)


if __name__ == '__main__':
    main()
