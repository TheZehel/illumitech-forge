#!/usr/bin/env python3
"""Read-only review/workbench protocol fixture; no provider inference or acceptance."""
import json
import sys
from pathlib import Path
for line in sys.stdin:
    v = json.loads(line)
    method = v.get('method')
    p = v.get('params', {})
    if 'id' not in v:
        continue
    def answer(result):
        print(json.dumps({'id': v['id'], 'result': result}), flush=True)
    def event(method, params):
        print(json.dumps({'method': method, 'params': params}), flush=True)
    if method == 'initialize':
        answer({'userAgent': 'local-review-fixture'})
    elif method == 'account/read':
        answer({'account': {'type': 'chatgpt'}})
    elif method == 'thread/start':
        assert 'model' not in p and 'config' not in p
        assert p['sandbox'] == 'read-only'
        assert p['approvalPolicy'] in ('never', 'on-request')
        assert Path(p['cwd']).resolve() == Path.cwd()
        answer({'thread': {'id': 'review-thread', 'cwd': str(Path.cwd()), 'turns': []}, 'model': 'fixture-configured', 'sandbox': {'type': 'readOnly'}, 'approvalPolicy': p['approvalPolicy']})
    elif method == 'turn/start':
        answer({'turn': {'id': 'review-turn'}})
        event('thread/tokenUsage/updated', {'threadId': 'review-thread', 'tokenUsage': {'total': {'inputTokens': 20, 'cachedInputTokens': 5, 'outputTokens': 3}}})
        event('item/completed', {'threadId': 'review-thread', 'item': {'type': 'agentMessage', 'text': '{"schema_version":1,"findings":[]}'}})
        event('turn/completed', {'threadId': 'review-thread', 'turn': {'id': 'review-turn', 'status': 'completed'}})
    elif method == 'turn/interrupt':
        answer({})
    else:
        print(json.dumps({'id': v['id'], 'error': {'code': -32601, 'message': 'Unsupported fixture method'}}), flush=True)
