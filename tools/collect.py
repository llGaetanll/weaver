"""Catch the one trace a page posts, write it to stdout, and stop.

A page cannot hand anything back to the shell that started the browser, so it
posts instead. One request is all there is to serve.
"""

import http.server
import sys


class Catch(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("content-length", 0))

        sys.stdout.write(self.rfile.read(length).decode())
        sys.stdout.flush()

        self.send_response(204)
        self.send_header("access-control-allow-origin", "*")
        self.end_headers()

    def log_message(self, *args):
        pass


http.server.HTTPServer(("127.0.0.1", int(sys.argv[1])), Catch).handle_request()
