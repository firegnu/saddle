#!/usr/bin/env python3
"""Synthetic Attention source; all control files live in an isolated test directory."""
import json, sys, pathlib

def save(name, value):
    path = pathlib.Path(name)
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value))
    temporary.replace(path)

def emit(m):
    print(json.dumps(m), flush=True)

for line in sys.stdin:
    m = json.loads(line)
    if m['kind'] == 'request':
        if m['method'] == 'initialize':
            emit({'kind':'response','id':m['id'],'result':{'id':'test.attention','version':'1','protocol_major':1,'capabilities':['panel.v1','ui.entry.v1','attention.v1'],'width_profile':'saddle-grapheme-v1'}})
            emit({'kind':'request','id':1,'method':'attention.replace','params':{'items':[{'id':'one','title':'Synthetic item','note':'Only a demo','action':'open','target':{'document':'one'}}]}})
        elif m['method'] == 'shutdown':
            break
        else:
            emit({'kind':'response','id':m['id'],'result':{}})
    elif m['kind'] == 'response':
        save('response.json', m)
    elif m['name'] == 'attention.open':
        save('opened.json', m['data'])
    elif m['name'] == 'test.replace':
        emit({'kind':'request','id':m['data']['id'],'method':'attention.replace','params':m['data']['snapshot']})
    elif m['name'] == 'test.exit':
        break
