import copy
import contextlib
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import patch

import memory_migration as m


class MigrationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve()
        self.source = self.root / 'source'
        self.source.mkdir()
        with contextlib.closing(sqlite3.connect(self.source / 'memory.db')) as db:
            db.execute('CREATE TABLE pages(id TEXT, scope TEXT, body TEXT)')
            db.executemany('INSERT INTO pages VALUES(?,?,?)', [('a', 'one', 'alpha'), ('b', 'two', 'beta')])
            db.commit()
        (self.source / 'config.toml').write_text('private = "fixture"\n')
        (self.source / 'corpus').mkdir()
        (self.source / 'corpus/page.md').write_text('fixture content')
        self.binary = self.root / 'binary'
        self.binary.write_text('fixture binary')
        self.plan = dict(schema=1, storage_root=str(self.root), backup=str(self.root / 'backup'),
                         native=str(self.root / 'native'), receipts=str(self.root / 'receipts'),
                         binary=str(self.binary), sha256=m.digest(self.binary), version='ai-memory fixture',
                         version_args=['--version'], endpoint='http://127.0.0.1:49190/mcp',
                         production_endpoint='http://127.0.0.1:49191/mcp', container='fixture', volume='memory',
                         data_mount='/data', database='memory.db', config='config.toml', server_version='fixture',
                         tables={'pages': ['id', 'scope', 'body']},
                         probes=[dict(workspace='fixture', project=p, query=i, ids=[i], counts={'pages': 1},
                                      pages={i: {'path': i + '.md', 'sha256': 'unused'}})
                                 for p, i in [('one', 'a'), ('two', 'b')]])
        self.plan.update(isolation_evidence=str(self.root / 'isolation.json'),
                         shutdown_evidence=str(self.root / 'shutdown.json'),
                         transition_approval=str(self.root / 'approval.json'))
        self.inv = dict(container_id='fixture', image_id='sha256:fixture', running=False, writers=[])

    def load(self):
        path = self.root / 'plan.json'
        path.write_text(json.dumps(self.plan))
        return m.load_plan(path)

    def backup(self):
        def docker(*args):
            self.assertEqual(args[0], 'cp')
            import shutil
            shutil.copytree(self.source, Path(args[-1]), dirs_exist_ok=True)
        with patch.object(m, 'stopped', return_value=self.inv), patch.object(m, 'docker', side_effect=docker):
            m.backup(self.plan)

    def restore(self):
        self.backup()
        with patch.object(m, 'check_version'):
            m.rehearse(self.plan)

    def test_preview_no_process_no_writes(self):
        plan = self.load()
        before = m.files(self.root)
        with patch.object(m.subprocess, 'run', side_effect=AssertionError('executed')):
            for phase in ['inventory', 'backup', 'rehearse', 'verify', 'finish', 'stop-writes', 'cutover', 'rollback']:
                self.assertEqual(m.run(plan, phase)['status'], 'preview')
        self.assertEqual(m.files(self.root), before)

    def test_bad_checksum_scope_endpoint_overlap_symlink(self):
        original = copy.deepcopy(self.plan)
        for key, value in [('sha256', 'bad'), ('endpoint', self.plan['production_endpoint']),
                           ('native', self.plan['backup']), ('endpoint', 'http://example.invalid:80/mcp'),
                           ('database', '../live.db')]:
            self.plan = {**original, key: value}
            with self.assertRaises(ValueError):
                self.load()
        self.plan = original
        (self.root / 'link').symlink_to(self.source, target_is_directory=True)
        self.plan['native'] = str(self.root / 'link/new')
        with self.assertRaises(ValueError):
            self.load()

    def test_backup_restore_preserves_all_source_bytes(self):
        before = m.files(self.source)
        self.restore()
        self.assertEqual(m.files(self.source), before)
        self.assertEqual(m.files(Path(self.plan['backup']) / 'data'), before)
        native = Path(self.plan['native'])
        self.assertFalse((native / 'config.toml').exists())
        self.assertEqual((native / 'config.toml.production-disabled').read_bytes(), (self.source / 'config.toml').read_bytes())
        self.assertEqual(m.read_receipt(self.plan, 'rehearse')['baseline']['tables']['pages']['count'], 2)

    def test_existing_destinations_are_preserved(self):
        self.restore()
        before = m.files(self.root)
        with self.assertRaises(ValueError):
            self.backup()
        with patch.object(m, 'check_version'), self.assertRaises(ValueError):
            m.rehearse(self.plan)
        self.assertEqual(m.files(self.root), before)

    def test_active_writer_and_inventory_change_refused(self):
        with patch.object(m, 'inventory', return_value={**self.inv, 'writers': ['other']}):
            with self.assertRaises(ValueError):
                m.backup(self.plan)
        self.assertFalse(Path(self.plan['backup']).exists())
        with patch.object(m, 'stopped', side_effect=[self.inv, {**self.inv, 'started': 'changed'}]), patch.object(m, 'docker'):
            with self.assertRaises(ValueError):
                m.backup(self.plan)
        self.assertFalse(Path(self.plan['receipts']).exists())

    def test_corrupt_backup_refused_before_restore(self):
        self.backup()
        (Path(self.plan['backup']) / 'data/memory.db').write_bytes(b'corrupt')
        with patch.object(m, 'check_version'), self.assertRaises(ValueError):
            m.rehearse(self.plan)
        self.assertFalse(Path(self.plan['native']).exists())

    def test_failed_restore_never_creates_success_receipt(self):
        self.backup()
        before = m.files(self.source)
        with patch.object(m, 'check_version'), patch.object(m, 'offline', side_effect=ValueError('integrity')):
            with self.assertRaises(ValueError):
                m.rehearse(self.plan)
        self.assertFalse((Path(self.plan['receipts']) / 'rehearse.json').exists())
        self.assertEqual(m.files(self.source), before)
        with patch.object(m, 'check_version'), self.assertRaises(ValueError):
            m.rehearse(self.plan)  # Partial restore must not be overwritten on retry.

    def test_version_mismatch(self):
        with patch.object(m.subprocess, 'run') as run:
            run.return_value.stdout = b'wrong version'
            with self.assertRaises(ValueError):
                m.check_version(self.plan)
            self.assertNotIn('AI_MEMORY_SERVER_URL', run.call_args.kwargs['env'])

    def test_unmounted_storage_and_concurrent_lock(self):
        with patch.object(m.os.path, 'ismount', return_value=False), self.assertRaises(ValueError):
            m.run(self.plan, 'backup', True)
        with m.lock(self.plan), self.assertRaises(BlockingIOError):
            with m.lock(self.plan):
                self.fail('second writer acquired lock')

    def test_rollback_requires_verified_final_snapshot_and_no_new_writes(self):
        self.restore()
        approval_path = self.root / 'approval.json'
        self.plan['transition_approval'] = str(approval_path)
        # Fresh external receipts bound to the final plan.
        with patch.object(m, 'read_receipt', return_value={'manifest': 'final'}), patch.object(m, 'checked_backup'):
            approval = dict(plan=m.canonical(self.plan), phase='rollback', operational_authorization=True,
                            writers=[], final_backup_verified=True, fresh_client_verified=True,
                            final_backup_manifest='final', new_writes=True, old_binary_compatible=True,
                            commands=[['operator-reviewed-old-server', 'preserved-copy']])
            approval_path.write_text(json.dumps(approval))
            with self.assertRaises(ValueError):
                m.transition(self.plan, 'rollback')
            approval['new_writes'] = False
            approval_path.write_text(json.dumps(approval))
            with patch.object(m.subprocess, 'run', side_effect=AssertionError('production command executed')):
                self.assertFalse(m.transition(self.plan, 'rollback')['executed'])

    def test_post_migration_content_drift_blocks_finish(self):
        self.plan['shutdown_evidence'] = str(self.root / 'shutdown.json')
        self.restore()
        m.receipt(self.plan, 'mcp', {'scopes': 2})
        Path(self.plan['shutdown_evidence']).write_text(json.dumps(
            {'plan': m.canonical(self.plan), 'writers': [], 'stopped': True}))
        native = Path(self.plan['native'])
        with contextlib.closing(sqlite3.connect(native / 'memory.db')) as db:
            db.execute("UPDATE pages SET body='changed' WHERE id='a'")
            db.commit()
        with patch('socket.socket') as socket:
            socket.return_value.__enter__.return_value.connect_ex.return_value = 1
            with self.assertRaises(ValueError):
                m.finish(self.plan)
        self.assertFalse((Path(self.plan['receipts']) / 'verified.json').exists())

    def test_snapshot_symlinks_and_hardlinks_are_refused(self):
        link = self.source / 'alias'
        link.symlink_to(self.binary)
        with self.assertRaises(ValueError):
            m.files(self.source)
        link.unlink()
        link.hardlink_to(self.binary)
        with self.assertRaises(ValueError):
            m.files(self.source)

    def test_finish_binds_offline_and_mcp_receipts(self):
        self.plan['shutdown_evidence'] = str(self.root / 'shutdown.json')
        self.restore()
        m.receipt(self.plan, 'mcp', {'scopes': 2})
        Path(self.plan['shutdown_evidence']).write_text(json.dumps(
            {'plan': m.canonical(self.plan), 'writers': [], 'stopped': True}))
        with patch('socket.socket') as socket:
            socket.return_value.__enter__.return_value.connect_ex.return_value = 1
            self.assertEqual(m.finish(self.plan)['status'], 'rehearsal-verified')
        self.assertEqual(m.read_receipt(self.plan, 'verified')['projections']['tables']['pages']['count'], 2)

    def test_inventory_redacts_values_and_detects_shared_writer(self):
        raw = {'Id': 'source', 'Image': 'sha256:fixture',
               'HostConfig': {'RestartPolicy': {'Name': 'no'}},
               'State': {'Running': False, 'StartedAt': 'start', 'FinishedAt': 'stop'},
               'Config': {'Env': ['API_KEY=do-not-output-this']},
               'Mounts': [{'Destination': '/data', 'Type': 'volume', 'Name': 'memory',
                           'Source': '/docker-private-volume', 'RW': True}]}
        other = {**raw, 'Id': 'other'}
        with patch.object(m, 'docker', side_effect=[json.dumps([raw]).encode(), b'other', json.dumps([other]).encode()]):
            result = m.inventory(self.plan)
        self.assertEqual(result['writers'], ['other'])
        self.assertEqual(result['environment_keys'], ['API_KEY'])
        self.assertNotIn('do-not-output-this', json.dumps(result))

    def test_invalid_sqlite_snapshot_cannot_pass_restore(self):
        (self.source / 'memory.db').write_bytes(b'not a database')
        self.backup()
        with patch.object(m, 'check_version'), self.assertRaises(sqlite3.DatabaseError):
            m.rehearse(self.plan)
        self.assertFalse((Path(self.plan['receipts']) / 'rehearse.json').exists())

    def test_mcp_scope_and_id_failures(self):
        class Client:
            bad = False
            bad_scope = False
            bad_page = False
            legacy_page = False
            bad_body = False
            def rpc(self, method, params, notification=False):
                if method == 'initialize':
                    return {'serverInfo': {'version': 'fixture'}}
                if method == 'tools/list':
                    return {'tools': [{'name': n} for n in ['memory_query', 'memory_briefing', 'memory_read_page']]}
            def call(self, name, args):
                if name == 'memory_briefing':
                    return {'scope': {**args, 'resolved_by': 'explicit',
                                      'project': 'wrong' if self.bad_scope else args['project']}, 'pages': 1}
                if name == 'memory_query':
                    expected = {'one': 'a', 'two': 'b'}[args['project']]
                    return {'hits': [{'id': 'wrong' if self.bad else args['query']}] if args['query'] == expected else []}
                return {'id' if self.legacy_page else 'page_id': 'wrong' if self.bad_page else args['path'][0],
                        'body': 'changed' if self.bad_body else 'fixture'}
        for probe in self.plan['probes']:
            for page in probe['pages'].values():
                page['sha256'] = m.hashlib.sha256(b'fixture').hexdigest()
        client = Client()
        self.assertEqual(m.verify_mcp(self.plan, client)['scopes'], 2)
        for failure in ['bad', 'bad_scope', 'bad_page', 'legacy_page', 'bad_body']:
            with self.subTest(failure=failure):
                client = Client()
                setattr(client, failure, True)
                with self.assertRaises(ValueError):
                    m.verify_mcp(self.plan, client)


if __name__ == '__main__':
    unittest.main()
