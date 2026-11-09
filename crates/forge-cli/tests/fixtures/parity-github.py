#!/usr/bin/env python3
"""Disposable PR/release transport. All provider data lives beside the test binary shims."""
import json
import pathlib
import shutil
import subprocess
import sys
import time

r = pathlib.Path(__file__).resolve().parent.parent
a = sys.argv[1:]
with (r / 'gh-calls.jsonl').open('a') as log:
    log.write(json.dumps(a) + '\n')


def arg(name):
    return a[a.index(name) + 1]


def stop(point):
    marker = r / 'release-fail'
    if marker.exists() and marker.read_text() == point:
        marker.unlink()
        print('transport outcome unknown', file=sys.stderr)
        sys.exit(3)


if a[:2] == ['repo', 'view']:
    assert a[2] == 'github.com/fixture/project'
    print(json.dumps({'visibility': (r / 'visibility').read_text() if (r / 'visibility').exists() else 'PRIVATE'}))
elif a[:2] == ['pr', 'list']:
    if (r / 'block-list').exists():
        (r / 'list-waiting').touch()
        time.sleep(60)
    print((r / 'pr.json').read_text() if (r / 'pr.json').exists() else '[]')
elif a[:2] == ['pr', 'create']:
    p = {'url': 'https://github.com/fixture/project/pull/1', 'state': 'OPEN', 'isDraft': True,
         'baseRefName': 'main', 'headRefOid': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
         'headRefName': 'forge/pilot', 'headRepository': {'nameWithOwner': 'fixture/project'},
         'title': arg('--title'), 'body': pathlib.Path(arg('--body-file')).read_text()}
    (r / 'pr.json').write_text(json.dumps([p]))
    if (r / 'fail-create').exists():
        print('HTTP 504 after provider persisted draft', file=sys.stderr)
        sys.exit(3)
    print(p['url'])
else:
    state = r / 'release.json'
    release = json.loads(state.read_text()) if state.exists() else None
    data = r / 'release-files'
    data.mkdir(exist_ok=True)
    writes = r / 'release-writes.json'

    def save(operation):
        state.write_text(json.dumps(release))
        previous = json.loads(writes.read_text()) if writes.exists() else []
        writes.write_text(json.dumps(previous + [operation]))

    if a[0] == 'api':
        assert arg('--hostname') == 'github.com'
        endpoint = next(x for x in a if x.startswith('repos/fixture/project/'))
        if (r / 'release-read-error').exists():
            # A textual 404 on stderr is deliberately NOT evidence of absence.
            print('HTTP 404 or upstream unavailable', file=sys.stderr)
            sys.exit(4)
        if (r / 'release-block-read').exists():
            (r / 'release-read-waiting').touch()
            time.sleep(60)
        if '--method' in a:
            assert arg('--method') == 'DELETE'
            ident = int(endpoint.rsplit('/', 1)[1])
            item = next(x for x in release['assets'] if x['id'] == ident)
            assert item['state'] == 'starter' and item['size'] == 0
            stop('delete_before')
            release['assets'].remove(item)
            save('delete:' + item['name'])
            stop('delete_after')
        else:
            assert '--paginate' in a and '--slurp' in a
            if endpoint.endswith('/releases'):
                rows = [release] if release else []
                if (r / 'release-unrelated').exists():
                    rows.insert(0, {'id': 99, 'tag_name': 'v0.0.0', 'name': None, 'body': None})
                print(json.dumps([rows]))
            elif endpoint.endswith('/assets'):
                print(json.dumps([release['assets']]))
            else:
                raise AssertionError(endpoint)
    else:
        assert a[0] == 'release' and arg('--repo') == 'github.com/fixture/project'
        action = a[1]
        if action == 'create':
            assert release is None and '--draft' in a and '--verify-tag' in a
            assert pathlib.Path(arg('--notes-file')).is_absolute()
            stop('create_before')
            release = {'id': 1, 'tag_name': a[2], 'draft': True, 'prerelease': False,
                       'name': arg('--title'), 'body': pathlib.Path(arg('--notes-file')).read_text(),
                       'target_commitish': arg('--target'), 'assets': [],
                       'html_url': 'https://github.com/fixture/project/releases/tag/' + a[2]}
            save('create')
            stop('create_after')
            print(release['html_url'])
        elif action == 'upload':
            path = pathlib.Path(a[-1])
            assert path.is_absolute() and path.is_file() and release['draft']
            assert not any(x['name'] == path.name for x in release['assets'])
            stop('upload_before')
            item = {'id': max([x['id'] for x in release['assets']] + [0]) + 1,
                    'name': path.name, 'state': 'uploaded', 'size': path.stat().st_size}
            if (r / 'release-fail').exists() and (r / 'release-fail').read_text() == 'starter':
                item.update(state='starter', size=0)
            else:
                shutil.copyfile(path, data / path.name)
            release['assets'].append(item)
            save('upload:' + path.name)
            stop('starter')
            stop('upload_after')
        elif action == 'download':
            target = pathlib.Path(arg('--dir'))
            assert target.is_absolute() and not list(target.iterdir())
            for item in release['assets']:
                if item['state'] == 'uploaded' and not (r / 'release-omit-download').exists():
                    shutil.copyfile(data / item['name'], target / item['name'])
            if (r / 'release-extra-download').exists():
                (target / 'foreign.txt').write_bytes(b'foreign')
        elif action == 'edit':
            assert '--draft=false' in a and release['draft']
            stop('publish_before')
            release['draft'] = False
            save('publish')
            stop('publish_after')
        else:
            raise AssertionError(a)
