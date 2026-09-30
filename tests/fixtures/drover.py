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
for group, status in [('current', 'running'), ('awaiting', 'awaiting_release'), ('pending', 'pending'), ('history', 'done')]:
    tasks = state.get(group) or []
    if isinstance(tasks, dict): tasks = [tasks]
    for task in tasks:
        task.setdefault('status', status)
        task.setdefault('body', '')
        task.setdefault('actions', {})
if len(args) == 4 and args[0] == 'show' and args[2:] == ['--json', '--with-agent-status']:
    tasks = ([state['current']] if state.get('current') else []) + ([state['awaiting']] if state.get('awaiting') else []) + state.get('pending', []) + state.get('history', [])
    task = next(t for t in tasks if t.get('id') == args[1])
    print(json.dumps(dict(schema_version=2, ok=True, project=str(root), task=task,
        evidence=dict(scope='repository_reference', controls_transition=False, observed_at=1,
            git=dict(state='unavailable'), last_check=dict(state='unknown')))))
    sys.exit(0)
if args == ['list', '--json']:
    state.update(schema_version=2, ok=True, project=str(root))
    state.pop('mode', None)
    print(json.dumps(state))
    sys.exit(0)
elif args[0] in ('edit', 'move', 'drop') and (root / 'write-error').exists():
    print((root / 'write-error').read_text(), file=sys.stderr)
    sys.exit(10)
elif args == ['pause']:
    state['paused'] = True
elif args == ['resume']:
    state['paused'] = False
elif len(args) == 3 and args[0] == 'add':
    state['pending'].append(dict(id='T%d' % (len(state['pending']) + 1), title=args[1], body=args[2]))
elif len(args) == 4 and args[0] == 'edit':
    task = state['pending'][int(args[1]) - 1]
    task.update(title=args[2], body=args[3])
elif len(args) == 3 and args[0] == 'move':
    task = state['pending'].pop(int(args[1]) - 1)
    state['pending'].insert(int(args[2]) - 1, task)
elif len(args) == 4 and args[:2] == ['drop', '--pos']:
    task = state['pending'].pop(int(args[2]) - 1)
    task.update(status='dropped', reason=args[3])
    state['history'].append(task)
else:
    print('FORBIDDEN CLI: ' + repr(args), file=sys.stderr)
    sys.exit(99)
state_file.write_text(json.dumps(state))
print('operation completed')
