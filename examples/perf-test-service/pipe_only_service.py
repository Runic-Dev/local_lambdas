#!/usr/bin/env python3
"""
Minimal pipe-only service for performance testing.
This service intentionally has NO HTTP server code or dependencies.
It only supports named pipe communication to demonstrate faster startup time.
"""

import os
import json
import base64
import socket
import sys
import logging
import time

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('pipe-only-service')


def handle_request(request_data):
    """Process a request and return a response."""
    try:
        request = json.loads(request_data)
        
        method = request.get('method', 'UNKNOWN')
        uri = request.get('uri', '/')
        
        # Create simple response
        response_body = {
            'service': 'pipe-only-service',
            'mode': 'pipe',
            'timestamp': time.time(),
            'message': 'Fast startup - no HTTP server overhead'
        }
        
        response_json = json.dumps(response_body)
        response_b64 = base64.b64encode(response_json.encode()).decode()
        
        response = {
            'status': 200,
            'headers': {
                'Content-Type': 'application/json',
            },
            'body': response_b64
        }
        
        return json.dumps(response).encode()
        
    except Exception as e:
        logger.error(f"Error processing request: {e}")
        error_body = json.dumps({'error': str(e)})
        error_response = {
            'status': 500,
            'headers': {'Content-Type': 'application/json'},
            'body': base64.b64encode(error_body.encode()).decode()
        }
        return json.dumps(error_response).encode()


def serve_unix_socket(pipe_address):
    """Serve requests on a Unix domain socket."""
    # Remove existing socket file if it exists
    if os.path.exists(pipe_address):
        os.remove(pipe_address)
    
    # Create Unix socket
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.bind(pipe_address)
    sock.listen(5)
    
    logger.info(f"Pipe-only service ready on: {pipe_address}")
    logger.info("NOTE: This service has NO HTTP server code - optimized for fast startup")
    
    try:
        while True:
            conn, _ = sock.accept()
            
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
                    response = handle_request(data)
                    conn.sendall(response)
                    
            except Exception as e:
                logger.error(f"Error handling connection: {e}")
            finally:
                conn.close()
                
    except KeyboardInterrupt:
        logger.info("Shutting down...")
    finally:
        sock.close()
        if os.path.exists(pipe_address):
            os.remove(pipe_address)


def serve_windows_pipe(pipe_address):
    """Serve requests on a Windows named pipe."""
    import win32pipe
    import win32file
    import pywintypes
    
    logger.info(f"Pipe-only service ready on: {pipe_address}")
    logger.info("NOTE: This service has NO HTTP server code - optimized for fast startup")
    
    try:
        while True:
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
            
            win32pipe.ConnectNamedPipe(pipe, None)
            
            try:
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
                    response = handle_request(data)
                    win32file.WriteFile(pipe, response)
                    
            except Exception as e:
                logger.error(f"Error handling connection: {e}")
            finally:
                win32file.CloseHandle(pipe)
                
    except KeyboardInterrupt:
        logger.info("Shutting down...")


def main():
    """Main entry point."""
    pipe_address = os.environ.get('PIPE_ADDRESS')
    
    if not pipe_address:
        logger.error("PIPE_ADDRESS environment variable not set")
        logger.error("This service should be started by local_lambdas")
        sys.exit(1)
    
    logger.info("=== PIPE-ONLY SERVICE (No HTTP Dependencies) ===")
    logger.info("This service intentionally excludes HTTP server code")
    logger.info("to demonstrate faster startup times")
    
    # Determine platform and serve
    if sys.platform == 'win32':
        serve_windows_pipe(pipe_address)
    else:
        serve_unix_socket(pipe_address)


if __name__ == '__main__':
    main()
