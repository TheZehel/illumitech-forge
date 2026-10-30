#!/usr/bin/env python3
"""Operator-only native memory rehearsal. No live SQLite access or production execution."""
import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import urllib.request
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def canonical(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def safe_path(value):
    path = Path(value)
    require(path.is_absolute() and '..' not in path.parts, 'absolute normalized paths required')
    require(not any(p.is_symlink() for p in [path, *path.parents]), 'symlink path refused')
    return path.resolve()


def inside(path, parent):
    return path == parent or parent in path.parents


def relative(value):
    path = Path(value)
    require(value and not path.is_absolute() and '..' not in path.parts, 'unsafe relative path')
    return path


def files(root):
    result = {}
    for path in sorted(root.rglob('*')):
        require(not path.is_symlink(), 'snapshot symlink refused')
        if path.is_dir():
            continue
        require(path.is_file() and path.stat().st_nlink == 1, 'snapshot special file or hardlink refused')
        result[str(path.relative_to(root))] = digest(path)
    require(bool(result), 'empty snapshot refused')
    return result


def load_plan(path):
    plan_path = safe_path(str(Path(path).absolute()))
    require(not inside(plan_path, ROOT), 'operational plan must be outside repository')
    plan = json.loads(plan_path.read_text())
    require(plan['schema'] == 1, 'unsupported plan schema')
    storage = safe_path(plan['storage_root'])
    require(storage.is_dir() and storage != Path('/') and not inside(storage, ROOT), 'external storage outside repository required')
    paths = [safe_path(plan[k]) for k in ('backup', 'native', 'receipts')]
    for p in paths:
        require(inside(p, storage) and p != storage and not inside(p, ROOT), 'paths must be below selected storage')
    for i, p in enumerate(paths):
        for q in paths[i + 1:]:
            require(not inside(p, q) and not inside(q, p), 'overlapping destinations refused')
    for key in ('isolation_evidence', 'shutdown_evidence', 'transition_approval'):
        evidence = safe_path(plan[key])
        require(inside(evidence, storage) and not inside(evidence, ROOT)
                and not any(inside(evidence, p) for p in paths), 'evidence must be separate from runtime and backup')
    binary = safe_path(plan['binary'])
    require(binary.is_file() and digest(binary) == plan['sha256'], 'binary checksum mismatch')
    require(bool(plan['version']) and plan['version_args'] == ['--version'], 'exact version pin required')
    endpoint = urlsplit(plan['endpoint'])
    require(endpoint.scheme == 'http' and endpoint.hostname == '127.0.0.1' and endpoint.port
            and not endpoint.username and not endpoint.password and not endpoint.query and not endpoint.fragment,
            'explicit IPv4 loopback MCP endpoint required')
    production = urlsplit(plan['production_endpoint'])
    require(endpoint.port != production.port, 'rehearsal must use a separate port')
    require(plan['container'] and not plan['container'].startswith('-'), 'container ID required')
    require(plan['volume'] and plan['data_mount'].startswith('/') and '..' not in Path(plan['data_mount']).parts,
            'named data volume required')
    require(len(plan['probes']) == 2 and len({(p['workspace'], p['project']) for p in plan['probes']}) == 2,
            'two distinct explicit scopes required')
    ids = []
    for probe in plan['probes']:
        require(probe['workspace'] and probe['project'] and probe['query'] and probe['ids'], 'scoped ID probes required')
        ids.append(set(probe['ids']))
        require(probe['counts'] and set(probe['pages']) == set(probe['ids']), 'page content digests required for every ID')
    require(not ids[0] & ids[1], 'scope sentinels must be disjoint')
    relative(plan['database'])
    relative(plan['config'])
    require(bool(plan['tables']), 'offline ID/content projections required')
    return plan


def docker(*args):
    result = subprocess.run(['docker', *args], check=True, capture_output=True, timeout=120)
    return result.stdout


def inventory(plan):
    raw = json.loads(docker('inspect', plan['container']))[0]
    require(raw['HostConfig']['RestartPolicy']['Name'] in ('no', ''), 'disable restart policy under separate authorization first')
    mounts = raw['Mounts']
    selected = [m for m in mounts if m['Destination'] == plan['data_mount']]
    require(len(selected) == 1 and selected[0]['Type'] == 'volume'
            and selected[0]['Name'] == plan['volume'], 'data mount does not match named volume')
    # Other writable mounts would make this backup incomplete.
    require(all(m['Destination'] == plan['data_mount'] or not m.get('RW', True) for m in mounts),
            'additional writable mounts require a separate reviewed backup procedure')
    source = Path(selected[0]['Source'])
    storage = safe_path(plan['storage_root'])
    require(not inside(storage, source) and not inside(source, storage), 'storage overlaps source volume')
    writers = []
    for cid in docker('ps', '-q').decode().split():
        other = json.loads(docker('inspect', cid))[0]
        if any(m.get('Name') == plan['volume'] for m in other['Mounts']):
            writers.append(other['Id'])
    return {'container_id': raw['Id'], 'image_id': raw['Image'],
            'running': raw['State']['Running'], 'writers': writers,
            'started': raw['State'].get('StartedAt'), 'finished': raw['State'].get('FinishedAt'),
            'volume': plan['volume'], 'data_mount': plan['data_mount'],
            'environment_keys': sorted(v.split('=', 1)[0] for v in raw['Config'].get('Env', []) or []),
            'mounts': [{'type': m['Type'], 'destination': m['Destination'], 'writable': m.get('RW', True)} for m in mounts]}


def stopped(plan):
    inv = inventory(plan)
    require(not inv['running'] and not inv['writers'], 'active writer refused; stop writes separately under operator authorization')
    return inv


@contextlib.contextmanager
def lock(plan):
    # Outside the repository; protects operators using this tooling, not arbitrary server processes.
    import fcntl
    path = safe_path(plan['storage_root']) / '.memory-migration.lock'
    require(not path.is_symlink(), 'symlink lock refused')
    fd = os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        yield
    finally:
        os.close(fd)


def write_new(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, sort_keys=True, indent=2)
        stream.write('\n')


def receipt(plan, phase, value):
    root = safe_path(plan['receipts'])
    root.mkdir(mode=0o700, parents=False, exist_ok=True)
    write_new(root / (phase + '.json'), {'plan': canonical(plan), **value})


def read_receipt(plan, phase):
    value = json.loads(safe_path(str(safe_path(plan['receipts']) / (phase + '.json'))).read_text())
    require(value['plan'] == canonical(plan), 'receipt belongs to another plan')
    return value


def backup(plan):
    target = safe_path(plan['backup'])
    require(not target.exists(), 'backup destination already exists')
    before = stopped(plan)
    target.mkdir(mode=0o700)
    data = target / 'data'
    data.mkdir(mode=0o700)
    # A stopped container yields the complete corpus, config, SQLite and WAL together.
    docker('cp', plan['container'] + ':' + plan['data_mount'].rstrip('/') + '/.', str(data))
    after = stopped(plan)
    require(before == after, 'source inventory changed during backup')
    require((data / relative(plan['config'])).is_file(), 'configuration missing from snapshot')
    require((data / relative(plan['database'])).is_file(), 'database missing from snapshot')
    manifest = {'inventory': before, 'files': files(data)}
    for path in data.rglob('*'):
        path.chmod(0o700 if path.is_dir() else 0o600)
    write_new(target / 'manifest.json', manifest)
    receipt(plan, 'backup', {'manifest': canonical(manifest)})
    return {'status': 'backup-created', 'files': len(manifest['files'])}


def checked_backup(plan):
    base = safe_path(plan['backup'])
    safe_path(str(base / 'data'))
    manifest = json.loads(safe_path(str(base / 'manifest.json')).read_text())
    require(canonical(manifest) == read_receipt(plan, 'backup')['manifest'], 'backup receipt mismatch')
    require(files(base / 'data') == manifest['files'], 'backup hashes changed')
    return base / 'data'


def quote(name):
    return '"' + name.replace('"', '""') + '"'


def offline(plan, data):
    # Callers only supply the freshly restored isolated copy, never Docker volume paths.
    path = safe_path(str(data / relative(plan['database'])))
    require(path.is_file() and path.stat().st_nlink == 1, 'isolated database missing')
    with contextlib.closing(sqlite3.connect(path.as_uri() + '?mode=ro', uri=True)) as db:
        require(db.execute('PRAGMA integrity_check').fetchall() == [('ok',)], 'SQLite integrity failed')
        result = {'schema_version': db.execute('PRAGMA user_version').fetchone()[0], 'tables': {}}
        for table, columns in plan['tables'].items():
            require(columns and all(isinstance(c, str) for c in columns), 'table projection required')
            rows = db.execute('SELECT ' + ','.join(map(quote, columns)) + ' FROM ' + quote(table)).fetchall()
            encoded = [json.dumps(row, default=lambda b: {'bytes': b.hex()}, separators=(',', ':')) for row in rows]
            result['tables'][table] = {'count': len(rows), 'sha256': canonical(sorted(encoded))}
    return result


def check_version(plan):
    with tempfile.TemporaryDirectory(prefix='memory-version-', dir=plan['storage_root']) as scratch:
        env = {'HOME': scratch, 'PATH': '/usr/bin:/bin', 'AI_MEMORY_DATA_DIR': scratch,
               'XDG_CONFIG_HOME': scratch, 'XDG_DATA_HOME': scratch}
        result = subprocess.run([plan['binary'], '--version'], cwd=scratch, env=env,
                                capture_output=True, timeout=15, check=True)
        require(result.stdout.decode().strip() == plan['version'], 'binary version mismatch')


def rehearse(plan):
    source = checked_backup(plan)
    target = safe_path(plan['native'])
    require(not target.exists(), 'native destination already exists')
    check_version(plan)
    shutil.copytree(source, target, symlinks=True)
    require(files(target) == files(source), 'restore hash mismatch')
    baseline = offline(plan, target)
    # Keep copied production config inert. The operator supplies reviewed native config separately.
    config = target / relative(plan['config'])
    disabled = config.with_name(config.name + '.production-disabled')
    require(not disabled.exists(), 'disabled config destination occupied')
    config.rename(disabled)
    receipt(plan, 'rehearse', {'baseline': baseline, 'backup': read_receipt(plan, 'backup')['manifest']})
    return {'status': 'restored-offline', 'native_start': 'pending isolated operator launch', 'baseline': baseline}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError('MCP redirects refused')


class MCP:
    def __init__(self, endpoint):
        self.endpoint, self.session, self.counter = endpoint, None, 0
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())

    def rpc(self, method, params, notification=False):
        self.counter += 1
        payload = {'jsonrpc': '2.0', 'method': method, 'params': params}
        if not notification:
            payload['id'] = self.counter
        headers = {'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream',
                   'MCP-Protocol-Version': '2024-11-05'}
        if self.session:
            headers['Mcp-Session-Id'] = self.session
        request = urllib.request.Request(self.endpoint, json.dumps(payload).encode(), headers)
        with self.opener.open(request, timeout=20) as response:
            self.session = response.headers.get('Mcp-Session-Id', self.session)
            body = response.read(262145)
        require(len(body) <= 262144, 'MCP response too large')
        if notification:
            return None
        try:
            value = json.loads(body)
        except json.JSONDecodeError:
            values = [json.loads(line[5:]) for line in body.decode().splitlines() if line.startswith('data:')]
            value = next((v for v in values if v.get('id') == self.counter), {})
        require(value.get('jsonrpc') == '2.0' and value.get('id') == self.counter
                and 'error' not in value and 'result' in value, 'MCP response failed')
        return value['result']

    def call(self, name, arguments):
        result = self.rpc('tools/call', {'name': name, 'arguments': arguments})
        require(not result.get('isError'), 'MCP tool failed')
        for content in result.get('content', []):
            if content.get('type') == 'text':
                return json.loads(content['text'])
        raise ValueError('MCP structured content missing')


def verify_mcp(plan, client):
    info = client.rpc('initialize', {'protocolVersion': '2024-11-05', 'capabilities': {},
                                   'clientInfo': {'name': 'illumitech-forge-rehearsal', 'version': '1'}})
    require(info['serverInfo']['version'] == plan['server_version'], 'MCP version mismatch')
    client.rpc('notifications/initialized', {}, notification=True)
    names = {t['name'] for t in client.rpc('tools/list', {})['tools']}
    require({'memory_query', 'memory_briefing', 'memory_read_page'} <= names, 'MCP tools missing')
    for probe in plan['probes']:
        scope = {k: probe[k] for k in ('workspace', 'project')}
        briefing = client.call('memory_briefing', scope)
        resolved = briefing.get('scope')
        require(isinstance(resolved, dict) and all(resolved.get(k) == v for k, v in scope.items()),
                'MCP scope mismatch')
        for pointer, expected in probe['counts'].items():
            value = briefing
            for key in pointer.split('/'):
                value = value[key]
            require(value == expected, 'MCP count mismatch')
        found = client.call('memory_query', {**scope, 'query': probe['query'], 'limit': 50})
        require({h['id'] for h in found['hits']} == set(probe['ids']), 'MCP ID/search isolation mismatch')
        for other in plan['probes']:
            if other is not probe:
                cross = client.call('memory_query', {**scope, 'query': other['query'], 'limit': 50})
                require(cross['hits'] == [], 'cross-scope sentinel leaked')
        for page_id, page in probe['pages'].items():
            payload = client.call('memory_read_page', {**scope, 'path': page['path']})
            require(payload.get('page_id') == page_id and isinstance(payload.get('body'), str)
                    and hashlib.sha256(payload['body'].encode()).hexdigest() == page['sha256'],
                    'MCP page identity/content mismatch')
    return {'scopes': 2, 'health': 'initialize + tools + scoped search + page content'}


def verify(plan):
    baseline = read_receipt(plan, 'rehearse')['baseline']
    isolation = json.loads(safe_path(plan['isolation_evidence']).read_text())
    require(isolation['plan'] == canonical(plan) and isolation['endpoint'] == plan['endpoint']
            and isolation['data_dir'] == plan['native'] and isolation['binary_sha256'] == plan['sha256']
            and isolation['single_writer'] is True and isolation['schedulers'] is False
            and isolation['external_llm'] is False and isolation['production_hooks'] is False
            and isolation['egress_denied'] is True, 'reviewed native isolation evidence required')
    checked_backup(plan)
    # Server must already be stopped for offline verification; MCP is checked separately first.
    result = verify_mcp(plan, MCP(plan['endpoint']))
    receipt(plan, 'mcp', result)
    return {**result, 'status': 'MCP-passed', 'offline_after_stop': 'pending', 'baseline': baseline}


def finish(plan):
    # Explicit operator evidence of native shutdown, generated by a supervised launcher.
    evidence = json.loads(safe_path(plan['shutdown_evidence']).read_text())
    require(evidence['plan'] == canonical(plan) and evidence['writers'] == [] and evidence['stopped'] is True,
            'native shutdown evidence required')
    import socket
    with socket.socket() as sock:
        sock.settimeout(1)
        require(sock.connect_ex(('127.0.0.1', urlsplit(plan['endpoint']).port)) != 0, 'native endpoint still active')
    read_receipt(plan, 'mcp')
    checked_backup(plan)
    current = offline(plan, safe_path(plan['native']))
    require(current['tables'] == read_receipt(plan, 'rehearse')['baseline']['tables'], 'restored IDs/content/counts changed')
    receipt(plan, 'verified', {'projections': current, 'shutdown': canonical(evidence)})
    return {'status': 'rehearsal-verified', 'production': 'untouched'}


def transition(plan, phase):
    """Render reviewed commands only. This tool deliberately never executes production commands."""
    read_receipt(plan, 'verified')
    checked_backup(plan)
    approval = json.loads(safe_path(plan['transition_approval']).read_text())
    require(approval['plan'] == canonical(plan) and approval['phase'] == phase
            and approval['operational_authorization'] is True, 'separate operational approval required')
    if phase == 'stop-writes':
        require(approval['exclusive_operator'] is True, 'exclusive operator authorization required')
    else:
        require(approval['writers'] == [] and approval['final_backup_verified'] is True
                and approval['fresh_client_verified'] is True, 'stop-writes/final-backup/client prerequisites missing')
        require(approval['final_backup_manifest'] == read_receipt(plan, 'backup')['manifest'], 'final backup must be this rehearsed snapshot')
    if phase == 'rollback':
        require(approval['new_writes'] is False and approval['old_binary_compatible'] is True,
                'rollback requires reconciled writes and compatible preserved old snapshot')
    require(approval['commands'] and all(isinstance(c, list) and c and all(isinstance(a, str) for a in c)
                                       for c in approval['commands']), 'reviewed argv commands required')
    receipt(plan, phase + '-commands', {'commands': approval['commands'], 'executed': False})
    return {'status': 'reviewed-commands-exported', 'executed': False}


def run(plan, phase, apply=False):
    if not apply:
        return {'status': 'preview', 'phase': phase, 'writes': False, 'executes': False,
                'requirements': ['pinned binary', 'external mounted storage', 'exclusive writer', 'new destinations'],
                'production_commands': 'never executed by this tool'}
    require(sys.platform in ('darwin', 'linux'), 'native migration supports macOS/Linux only')
    require(os.path.ismount(plan['storage_root']), 'selected external storage must be a mounted filesystem')
    with lock(plan):
        # The binary is never launched as a server automatically.
        require(digest(safe_path(plan['binary'])) == plan['sha256'], 'binary checksum changed')
        if phase == 'inventory':
            return inventory(plan)
        return {'backup': backup, 'rehearse': rehearse, 'verify': verify, 'finish': finish,
                'stop-writes': lambda p: transition(p, 'stop-writes'),
                'cutover': lambda p: transition(p, 'cutover'),
                'rollback': lambda p: transition(p, 'rollback')}[phase](plan)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['inventory', 'backup', 'rehearse', 'verify', 'finish', 'stop-writes', 'cutover', 'rollback'])
    parser.add_argument('--plan', required=True)
    parser.add_argument('--apply', action='store_true')
    args = parser.parse_args()
    os.umask(0o077)
    try:
        print(json.dumps(run(load_plan(args.plan), args.phase, args.apply), indent=2))
    except (ValueError, KeyError, OSError, sqlite3.Error, subprocess.SubprocessError) as error:
        # Do not emit subprocess output, database contents or operational paths.
        print(json.dumps({'status': 'refused', 'error_type': type(error).__name__}), file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
