#!/usr/bin/env python3
"""Disposable provider protocol fixture, never remote acceptance evidence."""
import json
import os
import pathlib
import sys

state = pathlib.Path(os.environ['FORGE_TEST_PROVIDER'])
a = sys.argv[1:]
with (state / 'calls').open('a') as log:
    log.write(json.dumps(a) + '\n')
if a[:2] == ['issue', 'view']:
    assert a[2] == 'https://github.com/example/project/issues/42'
    print((state / 'issue.json').read_text())
elif a[:2] == ['pr', 'list']:
    p = state / 'prs.json'
    print(p.read_text() if p.exists() else '[]')
elif a[:2] == ['pr', 'create']:
    assert not (state / 'created').exists(), 'duplicate PR creation'
    (state / 'created').write_text('once')
    body = pathlib.Path(a[a.index('--body-file') + 1]).read_text()
    pr = dict(url='https://github.com/example/project/pull/7', state='OPEN',
              isDraft=True, baseRefName='main', headRefName='feature/m08',
              headRefOid=(state / 'branch').read_text(),
              headRepository={'nameWithOwner': 'example/project'},
              title=a[a.index('--title') + 1], body=body)
    mode = (state / 'mode').read_text() if (state / 'mode').exists() else ''
    if mode == 'divergent':
        pr['headRefOid'] = 'a' * 40
    if mode != 'missing':
        (state / 'prs.json').write_text(json.dumps([pr]))
    if mode == 'timeout':
        print('HTTP 504 timeout after provider persisted the PR', file=sys.stderr)
        sys.exit(1)
    print(pr['url'])
else:
    raise RuntimeError('unexpected provider call: ' + repr(a))
