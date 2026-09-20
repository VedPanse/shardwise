"""Serve the diagram with device-local progress: python3 serve_diagram.py."""
import json
import os
from pathlib import Path
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from threading import Lock
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent
STATE = ROOT / '.shardwise' / 'architecture-progress.json'
LOCK = Lock()
IDS = {'client', 'api', 'graph', 'metadata', 'scheduler', 'kafka',
       'worker1', 'worker2', 'worker3', 'artifacts', 'commit', 'result',
       'sdk', 'dispatch', 'status'}


def read_state():
    if not STATE.exists():
        return None
    data = json.loads(STATE.read_text())
    if not isinstance(data, list) or any(not isinstance(x, str) or x not in IDS for x in data):
        raise ValueError('Invalid saved progress')
    return set(data)


def save_state(completed):
    STATE.parent.mkdir(parents=True, exist_ok=True)
    temporary = STATE.with_suffix('.tmp')
    with temporary.open('w') as file:
        json.dump(sorted(completed), file)
        file.flush()
        os.fsync(file.fileno())
    os.replace(temporary, STATE)


class Handler(BaseHTTPRequestHandler):
    def reply(self, status, data, content_type='application/json'):
        body = data if isinstance(data, bytes) else json.dumps(data).encode()
        self.send_response(status)
        self.send_header('Content-Type', content_type)
        self.send_header('Cache-Control', 'no-store')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        path = urlsplit(self.path).path
        try:
            if path in ('/', '/index.html'):
                self.reply(200, (ROOT / 'index.html').read_bytes(), 'text/html; charset=utf-8')
            elif path == '/api/progress':
                with LOCK:
                    completed = read_state()
                self.reply(200, {'completed': sorted(completed) if completed is not None else None})
            else:
                self.reply(404, {'error': 'Not found'})
        except (OSError, ValueError):
            self.reply(500, {'error': 'Could not read device progress'})

    def do_POST(self):
        if urlsplit(self.path).path != '/api/progress':
            self.reply(404, {'error': 'Not found'})
            return
        # Only the locally served page should be able to change this file.
        origin = self.headers.get('Origin')
        if origin and origin not in (f'http://127.0.0.1:{self.server.server_port}',
                                     f'http://localhost:{self.server.server_port}'):
            self.reply(403, {'error': 'Invalid origin'})
            return
        if self.headers.get('Content-Type') != 'application/json':
            self.reply(415, {'error': 'Expected application/json'})
            return
        try:
            length = int(self.headers.get('Content-Length', '0'))
            if not 0 < length <= 4096:
                raise ValueError()
            data = json.loads(self.rfile.read(length))
            if not isinstance(data, dict):
                raise ValueError()
            initializing = set(data) == {'initial'}
            if initializing:
                if not isinstance(data['initial'], list) or any(not isinstance(x, str) or x not in IDS for x in data['initial']):
                    raise ValueError()
            elif (set(data) != {'id', 'completed'} or not isinstance(data['id'], str)
                  or data['id'] not in IDS or type(data['completed']) is not bool):
                raise ValueError()
        except (ValueError, TypeError):
            self.reply(400, {'error': 'Invalid progress update'})
            return
        try:
            with LOCK:
                completed = read_state()
                if initializing:
                    # Import old browser progress only if no device file exists.
                    if completed is None:
                        completed = set(data['initial'])
                        save_state(completed)
                else:
                    completed = completed or set()
                    if data['completed']:
                        completed.add(data['id'])
                    else:
                        completed.discard(data['id'])
                    save_state(completed)
            self.reply(200, {'completed': sorted(completed)})
        except (OSError, ValueError):
            self.reply(500, {'error': 'Could not save device progress'})


if __name__ == '__main__':
    server = ThreadingHTTPServer(('127.0.0.1', 8765), Handler)
    print('Diagram: http://127.0.0.1:8765/index.html', flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
