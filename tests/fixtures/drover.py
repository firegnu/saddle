#!/usr/bin/env python3
"""Fake public data/action CLI; it intentionally has no UI or board command."""
import json
from pathlib import Path
import sys
root = Path(__file__).parent
args = sys.argv[1:]
with (root / 'queue-events').open('a') as f:
    f.write(json.dumps(args, ensure_ascii=False) + '\n')
state_file = root / 'queue-state.json'
if state_file.exists():
    state = json.loads(state_file.read_text())
else:
    state = dict(mode={'loop': False, 'gate': True}, paused=False, current=None, awaiting=None,
                 pending=[dict(id='T1', title='Native queue task', body='\n'.join('detail line %d' % i for i in range(60)))], history=[])
if args == ['list', '--json']:
    print(json.dumps(state))
    sys.exit(0)
elif args == ['pause']:
    state['paused'] = True
elif args == ['resume']:
    state['paused'] = False
elif len(args) == 2 and args[0] == 'loop' and args[1] in ('on', 'off'):
    state['mode']['loop'] = args[1] == 'on'
elif len(args) == 3 and args[0] == 'add':
    state['pending'].append(dict(id='T%d' % (len(state['pending']) + 1), title=args[1], body=args[2]))
elif args == ['go']:
    print('checked public criteria')
    sys.exit(0)
elif args == ['next']:
    print('next request accepted')
    sys.exit(0)
else:
    print('FORBIDDEN CLI: ' + repr(args), file=sys.stderr)
    sys.exit(99)
state_file.write_text(json.dumps(state))
print('operation completed')
