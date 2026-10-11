#!/usr/bin/env python3
"""Serve built documentation with precompressed WASM on a Tailscale interface."""
import argparse
import functools
import re
import email.utils
import http.server
import json
import mimetypes
import subprocess
from pathlib import Path

class Handler(http.server.SimpleHTTPRequestHandler):
    def send_head(self):
        original = Path(self.translate_path(self.path))
        if not original.is_file():
            return super().send_head()
        accepted = {}
        for item in self.headers.get('Accept-Encoding', '').split(','):
            fields = item.strip().split(';')
            try:
                accepted[fields[0]] = next((float(v.strip()[2:]) for v in fields[1:] if v.strip().startswith('q=')), 1.)
            except ValueError:
                accepted[fields[0]] = 0.
        for encoding, extension in [('br', '.br'), ('gzip', '.gz')]:
            compressed = original.with_name(original.name + extension)
            if accepted.get(encoding, accepted.get('*', 0.)) <= 0 or not compressed.is_file():
                continue
            stamp=original.stat().st_mtime
            modified=self.headers.get('If-Modified-Since')
            try:
                fresh=modified and stamp < email.utils.parsedate_to_datetime(modified).timestamp()+1
            except (TypeError,ValueError,OverflowError):
                fresh=False
            if fresh:
                self.send_response(304);self.send_header('Vary','Accept-Encoding');self.end_headers();return None
            stream = compressed.open('rb')
            self.send_response(200)
            self.send_header('Content-Type', 'application/wasm' if original.suffix == '.wasm' else mimetypes.guess_type(original)[0] or 'application/octet-stream')
            self.send_header('Content-Encoding', encoding)
            self.send_header('Vary', 'Accept-Encoding')
            self.send_header('Cache-Control', 'public, max-age=31536000, immutable' if re.search(r'-[a-f0-9]{20}\.', original.name) else 'no-cache')
            self.send_header('Content-Length', str(compressed.stat().st_size))
            self.send_header('Last-Modified', self.date_time_string(original.stat().st_mtime))
            self.end_headers()
            return stream
        return super().send_head()

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bind', default=None)
    parser.add_argument('--port', type=int, default=8769)
    parser.add_argument('--directory', type=Path, default=Path(__file__).resolve().parents[1] / 'site/dist')
    args = parser.parse_args()
    host = args.bind or subprocess.check_output(['tailscale', 'ip', '-4'], text=True).strip().splitlines()[0]
    print(json.dumps({'url': f'http://{host}:{args.port}', 'directory': str(args.directory), 'compression': ['br', 'gzip']}), flush=True)
    http.server.ThreadingHTTPServer((host, args.port), functools.partial(Handler, directory=str(args.directory))).serve_forever()
