#!/usr/bin/env python3
"""Read-only, redacted public-release audit. Exit 1 on findings; never rewrites history."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
RULES = {
    'private_key': rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----',
    'credential_token': rb'(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|AKIA[A-Z0-9]{16}|sk-[A-Za-z0-9]{32,})',
    'personal_path': rb'(?:/Users/[^/\s"\']+/|/home/[^/\s"\']+/|[A-Za-z]:\\Users\\[^\\\s]+\\)',
    'internal_volume': rb'/Volumes/[^/\s]+/',
}


def scan(data):
    return [rule for rule, pattern in RULES.items() if re.search(pattern, data)]


def forbidden_path(path):
    p = Path(path)
    return any(x.startswith('.env') or x in {'.aws', '.ssh', 'credentials.json', 'auth.json'} for x in p.parts) or p.suffix in {'.db', '.sqlite', '.sqlite3', '.pem', '.p12', '.crate', '.exe'} or p.parts[0] in {'target', 'dist'}


def git(root, *args):
    return subprocess.check_output(['git', *args], cwd=root)


def audit(root):
    findings = []
    inventory = []
    def record(scope, identity, rules):
        for rule in rules:
            findings.append({'scope': scope, 'object': hashlib.sha256(identity).hexdigest(), 'rule': rule})
    # Include untracked nonignored files so preparation changes are covered before a commit.
    names = git(root, 'ls-files', '-z', '--cached', '--others', '--exclude-standard').split(b'\0')
    for raw in sorted(set(names) - {b''}):
        path = root / raw.decode()
        record('tree', raw, ['sensitive_or_artifact_path'] if forbidden_path(raw.decode()) else [])
        if path.is_symlink():
            record('tree', raw, ['symlink_review_required'])
            continue
        if path.is_file():
            data = path.read_bytes()
            record('tree', raw, scan(data))
            inventory.append({'path_sha256': hashlib.sha256(raw).hexdigest(), 'sha256': hashlib.sha256(data).hexdigest(), 'bytes': len(data)})
    # Scan every reachable blob, including deleted files and all local refs. No remote fetch.
    objects = git(root, 'rev-list', '--objects', '--all').splitlines()
    blobs = 0
    for row in objects:
        oid, _, path = row.partition(b' ')
        if git(root, 'cat-file', '-t', oid.decode()).strip() != b'blob':
            continue
        blobs += 1
        record('history', oid, scan(git(root, 'cat-file', 'blob', oid.decode())))
        if path and forbidden_path(path.decode()):
            record('history', oid, ['sensitive_or_artifact_path'])
    for workflow in (root / '.github/workflows').glob('*.y*ml'):
        text = workflow.read_text()
        if not re.search(r'^permissions:\s*\n  contents: read\s*$', text, re.M) or re.search(r':\s*write\b|write-all|pull_request_target', text):
            record('ci', workflow.name.encode(), ['permission_review_required'])
        if re.search(r'uses:\s*[^\s]+@(?![a-f0-9]{40}\b)', text):
            record('ci', workflow.name.encode(), ['unpinned_action'])
    meta = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--offline', '--format-version', '1'], cwd=root))
    expected = {(p['name'], p['version']) for p in meta['packages'] if p['source']}
    license_inventory = json.loads((root / 'docs/dependencies.json').read_text())
    actual = {(p['name'], p['version']) for p in license_inventory['dependencies']}
    if expected != actual or license_inventory['missing_notices']:
        record('licenses', b'inventory', ['incomplete_license_inventory'])
    for dep in license_inventory['dependencies']:
        if not dep['license'] or not dep['notices'] or any(not (root / 'THIRD_PARTY_LICENSES' / n).is_file() for n in dep['notices']):
            record('licenses', dep['name'].encode(), ['missing_notice'])
    if git(root, 'rev-parse', '--is-shallow-repository').strip() == b'true':
        record('history', b'shallow', ['incomplete_history'])
    return {'status': 'blocked' if findings else 'local_checks_passed', 'history_scope': 'all reachable local refs; no remote fetch or reflogs',
            'history_blobs': blobs, 'findings': findings, 'tree_inventory': inventory,
            'pending': ['human internal-data review', 'dedicated secret scanner over full fetched history', 'registry ownership/authentication', 'revision approval', 'native CI', 'publication'],
            'note': 'Heuristics cannot prove absence of secrets or private business data.'}


if __name__ == '__main__':
    report = audit(ROOT)
    print(json.dumps(report, indent=2))
    sys.exit(report['status'] == 'blocked')
