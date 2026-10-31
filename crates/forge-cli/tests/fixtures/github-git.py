#!/usr/bin/env python3
"""Only remote Git operations are simulated; local Git and evidence stay real."""
import json
import os
import pathlib
import sys

args = sys.argv[1:]
if args[0] in ['ls-remote', 'push']:
    state = pathlib.Path(os.environ['FORGE_TEST_PROVIDER'])
    with (state / 'git-calls').open('a') as log:
        log.write(json.dumps(args) + '\n')
    branch = state / 'branch'
    if args[0] == 'push':
        assert args == ['push', 'origin', 'HEAD:refs/heads/feature/m08']
        branch.write_text((state / 'commit').read_text())
    elif branch.exists():
        print(branch.read_text().strip() + '\trefs/heads/feature/m08')
else:
    os.execv('/usr/bin/git', ['git'] + args)
