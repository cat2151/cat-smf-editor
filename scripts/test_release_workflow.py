"""Offline regression checks for the Python steps in the reusable workflow.

Requires PyYAML (test-only); no network, git mutations, or release publication.
"""
import contextlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch
import zipfile

import yaml

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = yaml.safe_load(
    (ROOT / '.github/workflows/uapmd-windows-release.yml').read_text(encoding='utf-8')
)
SHA = 'a' * 40
OTHER_SHA = 'b' * 40


def step(job, name):
    return next(s['run'] for s in WORKFLOW['jobs'][job]['steps'] if s.get('name') == name)


@contextlib.contextmanager
def workspace():
    previous = Path.cwd()
    with tempfile.TemporaryDirectory() as temp:
        try:
            os.chdir(temp)
            yield Path(temp)
        finally:
            os.chdir(previous)


class ReleaseWorkflowTests(unittest.TestCase):
    def resolve(self, *, skip=True, body='', missing=False, error=None, sha=SHA, engine='1',
                dependency_shas=None):
        config = json.loads((ROOT / '.github/release.json').read_text(encoding='utf-8'))
        Path('release.json').write_text(json.dumps(config), encoding='utf-8')
        # Different app identity catches copied player/editor names in generic code.
        Path('Cargo.toml').write_text('[package]\nname="another-app"\nversion="2.3.4"\n', encoding='utf-8')
        Path('outputs').write_text('', encoding='utf-8')
        env = {'CONFIG': 'release.json', 'SKIP_UNCHANGED': str(skip).lower(),
               'GITHUB_SHA': SHA, 'ENGINE_REVISION': engine, 'GH_REPO': 'example/another-app',
               'GITHUB_OUTPUT': str(Path('outputs').resolve()),
               'GITHUB_STEP_SUMMARY': str(Path('summary').resolve())}
        response = subprocess.CompletedProcess([], 1 if missing or error else 0,
                                               json.dumps({'body': body}),
                                               error or ('HTTP 404' if missing else ''))
        def remote_sha(command, **kwargs):
            resolved = (dependency_shas or {}).get(command[-2], sha)
            return resolved + '\trefs/heads/main\n'

        with patch.dict(os.environ, env), patch('subprocess.check_output', side_effect=remote_sha), \
                patch('subprocess.run', return_value=response):
            exec(compile(step('resolve', 'Resolve dependencies and write notes'), '<resolve>', 'exec'), {})
        outputs = dict(line.split('=', 1) for line in Path('outputs').read_text(encoding='utf-8').splitlines())
        return outputs, Path('notes.md').read_text(encoding='utf-8')

    def test_first_release_and_generic_names(self):
        with workspace():
            out, notes = self.resolve(missing=True)
            self.assertEqual(out['build'], 'true')
            self.assertEqual(out['package'], 'another-app')
            self.assertIn('another-app-2.3.4-windows-x64.zip', notes)
            self.assertNotIn('cat-plugin-player-2.3.4-windows-x64.zip', notes)
            dependencies = json.loads(out['dependencies'])
            self.assertEqual([dep['path'] for dep in dependencies],
                             ['uapmd', 'cat-plugin-player', 'clap-mml-play-server',
                              'clap-mml-render-tui', 'cat-music-patterns', 'mascot-render-server'])
            self.assertTrue(all(dep['sha'] == SHA for dep in dependencies))
            self.assertIn('cat2151/cat-plugin-player', notes)
            self.assertIn(f'cat2151/clap-mml-play-server (refs/heads/main): {SHA}', notes)

    def test_all_cargo_path_dependencies_have_release_checkouts(self):
        manifest = tomllib.loads((ROOT / 'Cargo.toml').read_text(encoding='utf-8'))
        config = json.loads((ROOT / '.github/release.json').read_text(encoding='utf-8'))
        checkouts = {dep['path'] for dep in config['dependencies']}
        tables = [manifest]
        tables.extend(manifest.get('target', {}).values())
        for table in tables:
            for kind in ['dependencies', 'dev-dependencies', 'build-dependencies']:
                for name, dependency in table.get(kind, {}).items():
                    if not isinstance(dependency, dict) or 'path' not in dependency:
                        continue
                    with self.subTest(dependency=name):
                        parts = Path(dependency['path']).parts
                        self.assertEqual(parts[0], '..')
                        self.assertIn(parts[1], checkouts)

    def test_mascot_source_change_rebuilds_and_records_revision(self):
        config = json.loads((ROOT / '.github/release.json').read_text(encoding='utf-8'))
        sibling = next(dep for dep in config['dependencies']
                       if dep['path'] == 'mascot-render-server')
        self.assertEqual(sibling['repository'], 'cat2151/mascot-render-server')
        self.assertEqual(sibling['ref'], 'refs/heads/main')
        with workspace():
            _, body = self.resolve(missing=True)
            changed = {'https://github.com/cat2151/mascot-render-server': OTHER_SHA}
            out, notes = self.resolve(body=body, dependency_shas=changed)
            self.assertEqual(out['build'], 'true')
            self.assertIn(f'cat2151/mascot-render-server (refs/heads/main): {OTHER_SHA}', notes)
            out, _ = self.resolve(body=notes, dependency_shas=changed)
            self.assertEqual(out['build'], 'false')

    def test_cmrt_core_path_is_provided_by_release_checkout(self):
        manifest = tomllib.loads((ROOT / 'Cargo.toml').read_text(encoding='utf-8'))
        dependency = manifest['dependencies']['cmrt-core']
        self.assertEqual(dependency['path'], '../clap-mml-play-server/core-lib')
        self.assertIs(dependency['default-features'], False)
        config = json.loads((ROOT / '.github/release.json').read_text(encoding='utf-8'))
        sibling = next(dep for dep in config['dependencies']
                       if dep['path'] == Path(dependency['path']).parts[1])
        self.assertEqual(sibling['repository'], 'cat2151/clap-mml-play-server')
        self.assertEqual(sibling['ref'], 'refs/heads/main')

    def test_mml_converter_path_is_provided_and_its_changes_rebuild(self):
        manifest = tomllib.loads((ROOT / 'Cargo.toml').read_text(encoding='utf-8'))
        dependency = manifest['dependencies']['cmrt-chord']
        self.assertEqual(dependency['path'], '../clap-mml-render-tui/chord')
        config = json.loads((ROOT / '.github/release.json').read_text(encoding='utf-8'))
        sibling = next(dep for dep in config['dependencies']
                       if dep['path'] == Path(dependency['path']).parts[1])
        self.assertEqual(sibling['repository'], 'cat2151/clap-mml-render-tui')
        with workspace():
            _, body = self.resolve(missing=True)
            out, notes = self.resolve(body=body, dependency_shas={
                'https://github.com/cat2151/clap-mml-render-tui': OTHER_SHA})
            self.assertEqual(out['build'], 'true')
            self.assertIn(f'cat2151/clap-mml-render-tui (refs/heads/main): {OTHER_SHA}', notes)

    def test_corpus_checkout_and_corpus_only_change_rebuilds(self):
        config = json.loads((ROOT / '.github/release.json').read_text(encoding='utf-8'))
        corpus = next(dep for dep in config['dependencies'] if dep['path'] == 'cat-music-patterns')
        self.assertEqual(corpus['repository'], 'cat2151/cat-music-patterns')
        self.assertEqual(corpus['ref'], 'refs/heads/main')
        with workspace():
            _, body = self.resolve(missing=True)
            changed = {'https://github.com/cat2151/cat-music-patterns': OTHER_SHA}
            out, notes = self.resolve(body=body, dependency_shas=changed)
            self.assertEqual(out['build'], 'true')
            self.assertEqual([dep['sha'] for dep in json.loads(out['dependencies'])],
                             [SHA, SHA, SHA, SHA, OTHER_SHA, SHA])
            self.assertIn(f'cat2151/cat-music-patterns (refs/heads/main): {OTHER_SHA}', notes)
            out, _ = self.resolve(body=notes, dependency_shas=changed)
            self.assertEqual(out['build'], 'false')

    def test_unchanged_schedule_skips_but_manual_rebuilds(self):
        with workspace():
            _, body = self.resolve(missing=True)
            out, _ = self.resolve(body=body)
            self.assertEqual(out['build'], 'false')
            out, _ = self.resolve(body=body, skip=False)
            self.assertEqual(out['build'], 'true')

    def test_dependency_or_engine_change_rebuilds(self):
        with workspace():
            _, body = self.resolve(missing=True)
            for change in [{'sha': OTHER_SHA}, {'engine': '2'}]:
                out, _ = self.resolve(body=body, **change)
                self.assertEqual(out['build'], 'true')

    def test_auth_or_network_failure_is_not_missing_release(self):
        with workspace():
            for error in ['HTTP 403: forbidden', 'connection timeout']:
                with self.assertRaisesRegex(RuntimeError, error):
                    self.resolve(error=error)
                self.assertEqual(Path('outputs').read_text(encoding='utf-8'), '')

    def test_only_cmrt_source_change_rebuilds_schedule(self):
        with workspace():
            _, body = self.resolve(missing=True)
            out, notes = self.resolve(body=body, dependency_shas={
                'https://github.com/cat2151/clap-mml-play-server': OTHER_SHA})
            self.assertEqual(out['build'], 'true')
            dependencies = json.loads(out['dependencies'])
            self.assertEqual([dep['sha'] for dep in dependencies],
                             [SHA, SHA, OTHER_SHA, SHA, SHA, SHA])
            self.assertIn(f'cat2151/clap-mml-play-server (refs/heads/main): {OTHER_SHA}', notes)
            out, _ = self.resolve(body=notes, dependency_shas={
                'https://github.com/cat2151/clap-mml-play-server': OTHER_SHA})
            self.assertEqual(out['build'], 'false')

    def test_checkout_fetches_resolved_sha_not_moving_branch(self):
        dependencies = [{'repository': 'example/dependency', 'path': 'uapmd', 'sha': OTHER_SHA},
                        {'repository': 'example/shared-host', 'path': 'cat-plugin-player', 'sha': SHA},
                        {'repository': 'cat2151/clap-mml-play-server',
                         'path': 'clap-mml-play-server', 'sha': OTHER_SHA},
                        {'repository': 'cat2151/cat-music-patterns',
                         'path': 'cat-music-patterns', 'sha': OTHER_SHA},
                        {'repository': 'cat2151/mascot-render-server',
                         'path': 'mascot-render-server', 'sha': OTHER_SHA}]
        with patch.dict(os.environ, {'DEPENDENCIES': json.dumps(dependencies)}), patch('subprocess.run') as run:
            exec(compile(step('build', 'Checkout resolved sibling commits'), '<checkout>', 'exec'), {})
        commands = [call.args[0] for call in run.call_args_list]
        self.assertIn(['git', '-C', 'uapmd', 'fetch', '--depth=1', 'origin', OTHER_SHA], commands)
        self.assertIn(['git', '-C', 'uapmd', 'checkout', '--detach', 'FETCH_HEAD'], commands)
        self.assertIn(['git', '-C', 'cat-plugin-player', 'fetch', '--depth=1', 'origin', SHA], commands)
        self.assertIn(['git', '-C', 'cat-plugin-player', 'checkout', '--detach', 'FETCH_HEAD'], commands)
        self.assertIn(['git', '-C', 'clap-mml-play-server', 'fetch', '--depth=1', 'origin', OTHER_SHA], commands)
        self.assertIn(['git', '-C', 'clap-mml-play-server', 'checkout', '--detach', 'FETCH_HEAD'], commands)
        self.assertIn(['git', '-C', 'cat-music-patterns', 'fetch', '--depth=1', 'origin', OTHER_SHA], commands)
        self.assertIn(['git', '-C', 'cat-music-patterns', 'checkout', '--detach', 'FETCH_HEAD'], commands)
        self.assertIn(['git', '-C', 'mascot-render-server', 'fetch', '--depth=1', 'origin', OTHER_SHA], commands)
        self.assertIn(['git', '-C', 'mascot-render-server', 'checkout', '--detach', 'FETCH_HEAD'], commands)

    def package(self, files):
        env = {'PACKAGE': 'another-app', 'VERSION': '2.3.4', 'PACKAGE_FILES': json.dumps(files)}
        with patch.dict(os.environ, env):
            exec(compile(step('build', 'Package'), '<package>', 'exec'), {})

    def test_zip_has_only_configured_files_and_generic_exe(self):
        with workspace():
            Path('target/release').mkdir(parents=True)
            Path('target/release/another-app.exe').write_bytes(b'executable')
            Path('LICENSE').write_bytes(b'license')
            self.package(['LICENSE'])
            with zipfile.ZipFile('dist/another-app-2.3.4-windows-x64.zip') as archive:
                self.assertEqual(archive.namelist(), ['another-app.exe', 'LICENSE'])
                self.assertEqual(archive.read('another-app.exe'), b'executable')

    def test_missing_or_colliding_files_fail(self):
        with workspace():
            Path('target/release').mkdir(parents=True)
            Path('target/release/another-app.exe').write_bytes(b'executable')
            for files in [['missing'], ['../outside'], ['X:/outside'],
                          ['first/LICENSE', 'second/license']]:
                with self.assertRaises(ValueError):
                    self.package(files)


if __name__ == '__main__':
    unittest.main()
