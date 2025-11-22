#!/usr/bin/env python3
"""
Example child process that demonstrates the local_lambdas protocol.

This service:
1. Reads the pipe address from the PIPE_ADDRESS environment variable
2. Listens on the named pipe for HTTP requests
3. Echoes back request information in the response
"""

import os
import json
import base64
import socket
import sys
import logging
from pathlib import Path

# Configure logging
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('echo-service')


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
        
        logger.info(f"Received {method} request for {uri}")
        logger.debug(f"Headers: {headers}")
        logger.debug(f"Body: {body}")
        
        # Create echo response
        response_body = {
            'service': 'echo-service',
            'message': 'Request received successfully',
            'request': {
                'method': method,
                'uri': uri,
                'headers': dict(headers),
                'body': body
            }
        }
        
        response_json = json.dumps(response_body, indent=2)
        response_b64 = base64.b64encode(response_json.encode()).decode()
        
        # Create HTTP response
        response = {
            'status': 200,
            'headers': {
                'Content-Type': 'application/json',
                'X-Service': 'echo-service'
            },
            'body': response_b64
        }
        
        return json.dumps(response).encode()
        
    except Exception as e:
        logger.error(f"Error processing request: {e}", exc_info=True)
        
        # Return error response
        error_body = json.dumps({
            'error': str(e),
            'service': 'echo-service'
        })
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
    pipe_address = os.environ.get('PIPE_ADDRESS')
    
    if not pipe_address:
        logger.error("PIPE_ADDRESS environment variable not set")
        logger.error("This service should be started by local_lambdas")
        sys.exit(1)
    
    logger.info(f"Echo service starting...")
    logger.info(f"Pipe address: {pipe_address}")
    
    # Determine platform and serve accordingly
    if sys.platform == 'win32':
        serve_windows_pipe(pipe_address)
    else:
        serve_unix_socket(pipe_address)


if __name__ == '__main__':
    main()
