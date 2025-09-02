#!/usr/bin/env python3
"""
Simple HTTP server with proper WASM MIME type support
Usage: python3 wasm_server.py [port]
"""

import http.server
import socketserver
import sys
import mimetypes

# Add WASM MIME type
mimetypes.add_type('application/wasm', '.wasm')

class WASMHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        # Add CORS headers for local development
        self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
        self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
        super().end_headers()

if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
    
    with socketserver.TCPServer(("", port), WASMHandler) as httpd:
        print(f"🌐 Serving at http://localhost:{port}")
        print(f"📦 WASM files will be served with proper application/wasm MIME type")
        httpd.serve_forever()