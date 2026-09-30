#!/usr/bin/env python3
"""A deliberately small external peer for lifecycle/failure tests; no real agents."""
import json, os, sys, time
mode = sys.argv[1] if len(sys.argv) > 1 else 'normal'
for line in sys.stdin:
    m = json.loads(line)
    if m['kind'] != 'request':
        if mode in ('drain', 'delayed-drain') and m.get('name') == 'input':
            if 'payload' in m['data']:
                assert m['data']['payload'] == str(m['data']['input_id']) * 20000
            print('input:' + str(m['data']['input_id']), file=sys.stderr, flush=True)
        continue
    if m['method'] == 'initialize':
        assert all(k not in os.environ for k in ['CORRAL_NAME', 'CORRAL_INSTANCE', 'SADDLE_INSTANCE', 'SADDLE_PANE', 'SADDLE_REVISION'])
        assert not os.isatty(0) and os.getsid(0) == os.getpid()
        if mode == 'partial':
            sys.stdout.write('x' * (4 * 1024 * 1024 + 1)); sys.stdout.flush(); time.sleep(10)
        identity = 'wrong.id' if mode == 'wrong' else 'test.peer'
        print(json.dumps({'kind':'response','id':m['id'],'result':{'id':identity,'version':'1','protocol_major':1,'capabilities':['panel.v1','notify.v1'],'width_profile':'saddle-grapheme-v1'}}),flush=True)
        if mode == 'delayed-drain':
            time.sleep(0.3)
    elif m['method'] == 'shutdown':
        if mode == 'stubborn': time.sleep(10)
        break
    elif m['method'] == 'ping':
        print(json.dumps({'kind':'response','id':m['id'],'result':{}}),flush=True)
