#!/usr/bin/env python3
import json, sys
from pathlib import Path
root = Path(__file__).parent
with (root / 'starts').open('a') as f:
    f.write('start\n')
count = 0
size = None
frame_id = 0
for line in sys.stdin:
    m = json.loads(line)
    if m['kind'] == 'request':
        method = m['method']
        if method == 'shutdown':
            break
        result = {}
        if method == 'initialize':
            (root / 'initialize.json').write_text(json.dumps(m['params']))
            result = {'id':'test.entry','version':'1','protocol_major':1,'capabilities':['panel.v1','ui.entry.v1','panel.overlay.v1'],'width_profile':'saddle-grapheme-v1'}
        print(json.dumps({'kind':'response','id':m['id'],'result':result}),flush=True)
    if m['kind'] != 'event':
        continue
    data = m['data']
    name = m['name']
    dirty = False
    if name in ('panel.open','panel.resize'):
        size = data
        dirty = True
    if name == 'panel.close':
        size = None
    if name == 'input' and data['event'].get('code',{}).get('name') == 'enter' and data['event'].get('phase') == 'press':
        count += 1
        dirty = True
    if dirty and size and size['cols'] and size['rows_count']:
        frame_id += 1
        width = size['cols']
        rows = []
        for y in range(size['rows_count']):
            text = (f'Clicks: {count}' if y == 0 else '')[:width].ljust(width)
            rows.append([{'text':text,'fg':'default','bg':'default','modifiers':[]}])
        frame = dict(size,frame_id=frame_id,rows=rows)
        print(json.dumps({'kind':'event','name':'panel.frame','data':frame}),flush=True)
