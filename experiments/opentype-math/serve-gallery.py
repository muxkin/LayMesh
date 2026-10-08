#!/usr/bin/env python3
"""Serve only the generated math gallery on a Tailscale IPv4 interface."""
import argparse
from functools import lru_cache, partial
import gzip
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import io
import ipaddress
import json
from pathlib import Path
import subprocess
from urllib.parse import urlsplit

ROOT=Path(__file__).resolve().parents[2]


@lru_cache(maxsize=256)
def compressed(path, modified, size):
    return gzip.compress(Path(path).read_bytes(),compresslevel=5)


class Handler(SimpleHTTPRequestHandler):
    def list_directory(self,path):
        self.send_error(404,"No directory index")

    def end_headers(self):
        self.send_header("Cache-Control","no-cache")
        self.send_header("X-Content-Type-Options","nosniff")
        self.send_header("Referrer-Policy","no-referrer")
        self.send_header("Content-Security-Policy","default-src 'none'; script-src 'self'; connect-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; object-src 'none'; base-uri 'none'")
        super().end_headers()

    def send_head(self):
        if urlsplit(self.path).path=="/health":
            summary=json.loads((Path(self.directory)/"summary.json").read_text())
            payload=json.dumps({"ready":True,"cases":summary["case_entries"],"checks":summary["layout_checks"],
                "fonts":len(summary["fonts"]),"renderer_fingerprint":summary["renderer_fingerprint"]}).encode()
            self.send_response(200)
            self.send_header("Content-Type","application/json; charset=utf-8")
            self.send_header("Content-Length",str(len(payload)))
            self.end_headers()
            return io.BytesIO(payload)
        file=Path(self.translate_path(self.path))
        if file.is_file() and file.suffix in {".json",".js",".css",".svg"} and "gzip" in self.headers.get("Accept-Encoding",""):
            info=file.stat()
            payload=compressed(str(file),info.st_mtime_ns,info.st_size)
            self.send_response(200)
            self.send_header("Content-Type",self.guess_type(str(file)))
            self.send_header("Content-Encoding","gzip")
            self.send_header("Vary","Accept-Encoding")
            self.send_header("Content-Length",str(len(payload)))
            self.send_header("Last-Modified",self.date_time_string(info.st_mtime))
            self.end_headers()
            return io.BytesIO(payload)
        return super().send_head()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory",type=Path,default=ROOT/"target/opentype-math-gallery")
    parser.add_argument("--host",help="Default: current Tailscale IPv4 address")
    parser.add_argument("--port",type=int,default=8765)
    args=parser.parse_args()
    host=args.host or subprocess.check_output(["tailscale","ip","-4"],text=True).strip().splitlines()[0]
    if ipaddress.ip_address(host) not in ipaddress.ip_network("100.64.0.0/10"):
        parser.error("--host must be a Tailscale IPv4 address in 100.64.0.0/10")
    directory=args.directory.resolve()
    for file in ("index.html","app.js","styles.css","data.json","summary.json"):
        if not (directory/file).is_file(): parser.error(f"Missing {file}; run build-gallery.py first")
    server=ThreadingHTTPServer((host,args.port),partial(Handler,directory=str(directory)))
    print(f"Math gallery: http://{host}:{args.port}/",flush=True)
    try: server.serve_forever()
    except KeyboardInterrupt: pass
    finally: server.server_close()


if __name__=="__main__": main()
