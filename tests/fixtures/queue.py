#!/usr/bin/env python3
import os
from pathlib import Path
import signal
import sys
import tty
root = Path(__file__).parent

def log(text):
    with (root / 'queue-events').open('a') as f:
        f.write(text + '\n')

def stop(*_):
    log('stopped')
    sys.exit(0)

def resize(*_):
    s = os.get_terminal_size(0)
    log('size %dx%d' % (s.columns, s.lines))

tty.setraw(0)
signal.signal(signal.SIGINT, stop)
signal.signal(signal.SIGWINCH, resize)
resize()
os.write(1, b'QUEUE READY\r\n')
while True:
    data = os.read(0, 4096)
    if not data:
        break
    log('input ' + data.hex())
    if data == b'F':
        log('flood')
        while True:
            os.write(1, b'output ' * 512 + b'\r\n')
