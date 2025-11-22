#!/usr/bin/env python3
"""
HTTP-only service for performance testing.
This service uses Python's built-in HTTP server (no external dependencies like Flask/FastAPI).
It demonstrates the startup overhead of initializing a web server.
"""

import os
import json
import base64
import sys
import logging
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
import threading

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('http-only-service')


def handle_request(request_data):
    """Process a request and return a response."""
    try:
        request = json.loads(request_data)
        
        method = request.get('method', 'UNKNOWN')
        uri = request.get('uri', '/')
        
        # Create simple response
        response_body = {
            'service': 'http-only-service',
            'mode': 'http',
            'timestamp': time.time(),
            'message': 'Includes HTTP server initialization overhead'
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


class HTTPRequestHandler(BaseHTTPRequestHandler):
    """HTTP request handler."""
    
    def do_POST(self):
        """Handle POST requests."""
        try:
            # Read request body
            content_length = int(self.headers.get('Content-Length', 0))
            request_data = self.rfile.read(content_length)
            
            # Handle request
            response_data = handle_request(request_data)
            
            # Send response
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(response_data)))
            self.end_headers()
            self.wfile.write(response_data)
            
        except Exception as e:
            logger.error(f"Error handling HTTP request: {e}")
            self.send_error(500, str(e))
    
    def log_message(self, format, *args):
        """Suppress default HTTP server logs."""
        pass


def serve_http(address):
    """Serve requests via HTTP."""
    logger.info("=== HTTP-ONLY SERVICE (With Web Server) ===")
    logger.info("This service includes HTTP server initialization")
    logger.info("to demonstrate startup overhead")
    
    host, port_str = address.rsplit(':', 1)
    port = int(port_str)
    
    logger.info(f"Starting HTTP server on {host}:{port}...")
    
    # Simulate realistic HTTP server initialization time
    # (In real applications, this includes framework loading, middleware setup, etc.)
    server = HTTPServer((host, port), HTTPRequestHandler)
    
    logger.info(f"HTTP server ready on {host}:{port}")
    
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        logger.info("Shutting down HTTP server...")
    finally:
        server.server_close()


def main():
    """Main entry point."""
    http_address = os.environ.get('HTTP_ADDRESS')
    
    if not http_address:
        logger.error("HTTP_ADDRESS environment variable not set")
        logger.error("This service should be started by local_lambdas")
        sys.exit(1)
    
    serve_http(http_address)


if __name__ == '__main__':
    main()
