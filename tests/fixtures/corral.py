#!/usr/bin/env python3
"""Public CLI fake. All simulated data belongs to a per-test temporary directory."""
import json
import os
from pathlib import Path
import signal
import sys
import tty

root = Path(__file__).parent
verb = sys.argv[1]
name = sys.argv[2] if len(sys.argv) > 2 else ''

def log(text):
    with (root / 'events').open('a') as f:
        f.write(text + '\n')

def agents():
    return json.loads((root / 'agents.json').read_text())

log(verb + ' ' + name)
if verb == 'ls':
    print(json.dumps({'agents': [dict(name=n, cwd='/tmp/demo', instance='abcdef123', kind='claude') for n in agents()]}))
elif verb == 'status':
    if name not in agents():
        print(json.dumps({'ok': False, 'error': 'not_found'}))
        sys.exit(1)
    attached = int((root / name.replace('/', '-')).exists() or name == 'p/taken')
    print(json.dumps(dict(ok=True, name=name, instance='abcdef123', kind='claude', state=agents()[name], attached=attached, title='Synthetic title', last_input_source='human', last_output=100, turn_started=100)))
elif verb == 'reply':
    print(json.dumps(dict(ok=True, text='REPLY ' + name + '\n' + '\n'.join('line ' + str(i) for i in range(60)))))
elif verb == 'stop':
    state = agents()
    state.pop(name, None)
    (root / 'agents.json').write_text(json.dumps(state))
    print(json.dumps(dict(ok=True)))
elif verb == 'attach':
    marker = root / name.replace('/', '-')
    marker.write_text(str(os.getpid()))
    tty.setraw(0)
    def stop(*_):
        log('detached ' + name)
        marker.unlink(missing_ok=True)
        sys.exit(0)
    def resize(*_):
        size = os.get_terminal_size(0)
        log('size ' + name + ' %dx%d' % (size.columns, size.lines))
    signal.signal(signal.SIGINT, stop)
    signal.signal(signal.SIGWINCH, resize)
    resize()
    os.write(1, ('\x1b[?1000h\x1b[?1006h\x1b[?2004h\x1b[2J\x1b[H' + name + ' READY\r\n').encode())
    while True:
        data = os.read(0, 4096)
        if not data:
            break
        log('input ' + name + ' ' + data.hex())
        os.write(1, b'INPUT RECEIVED\r\n')
        if data == b'F':
            log('flood ' + name)
            while True:
                os.write(1, b'output ' * 512 + b'\r\n')
else:
    raise RuntimeError('unexpected command ' + verb)
