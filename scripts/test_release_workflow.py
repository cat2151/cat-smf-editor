"""Offline regression checks for the Python steps in the reusable workflow.

Requires PyYAML (test-only); no network, git mutations, or release publication.
"""
import contextlib
import json
import os
from pathlib import Path
import shutil
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
        Path('.github').mkdir(exist_ok=True)
        shutil.copyfile(ROOT / '.github/release-notes.md', '.github/release-notes.md')
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
            self.assertIn(f'cat-plugin-player: {SHA}', notes)
            self.assertIn(f'clap-mml-play-server: {SHA}', notes)

    def test_player_and_editor_generate_identical_common_prose(self):
        player = ROOT.parent / 'cat-plugin-player'
        self.assertEqual((ROOT / '.github/release-notes.md').read_bytes(),
                         (player / '.github/release-notes.md').read_bytes())
        workflow = yaml.safe_load((player / '.github/workflows/release.yml').read_text(encoding='utf-8'))
        renderer = next(s for s in workflow['jobs']['resolve']['steps']
                        if s.get('name') == 'Write the release notes')
        self.assertEqual(renderer['shell'], 'python')
        checkout = workflow['jobs']['resolve']['steps'][0]['with']['sparse-checkout']
        self.assertIn('.github/release-notes.md', checkout)
        verified = workflow['env']['UAPMD_VERIFIED_SHA']
        for sha in [verified, OTHER_SHA]:
            with self.subTest(sha=sha), workspace():
                _, editor_notes = self.resolve(missing=True, sha=sha)
                env = {'VERSION': '2.3.4', 'GITHUB_SHA': SHA, 'UAPMD': sha,
                       'UAPMD_VERIFIED_SHA': verified, 'TUI': sha, 'SERVER': sha,
                       'INPUTS': '<!-- build-inputs player-test -->',
                       'GITHUB_STEP_SUMMARY': str(Path('player-summary').resolve())}
                with patch.dict(os.environ, env):
                    exec(compile(renderer['run'], '<player-notes>', 'exec'), {})
                player_notes = Path('notes.md').read_text(encoding='utf-8')
                config = json.loads((ROOT / '.github/release.json').read_text(encoding='utf-8'))
                files = '、'.join(Path(file).name for file in config['package_files'])
                common = editor_notes.split('<details>')[0].replace('another-app', 'cat-plugin-player')
                common = common.replace('exe と ' + files + '。', 'exe と LICENSE。')
                self.assertEqual(common, player_notes.split('<details>')[0])
                uapmd_lines = lambda notes: [line for line in notes.splitlines()
                                             if 'uapmd' in line and not line.startswith('<!--')]
                self.assertEqual(uapmd_lines(editor_notes), uapmd_lines(player_notes))
                for notes in [editor_notes, player_notes]:
                    self.assertEqual(notes.count('<details>'), 1)
                    self.assertEqual(notes.count('</details>'), 1)
                    start, end = notes.index('<details>'), notes.index('</details>')
                    self.assertIn('<summary>ビルドに使った commit</summary>\n\n', notes[start:end])
                    self.assertIn(SHA, notes[start:end])
                    self.assertNotIn('<!-- build-inputs ', notes[:end])
                    self.assertIn('\n<!-- build-inputs ', notes[end:])
                    self.assertNotIn('Surge XT', notes)

    def test_distribution_build_and_tests_use_static_crt(self):
        self.assertEqual(WORKFLOW['jobs']['build']['env']['CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS'],
                         '-C target-feature=+crt-static')
        # A step-level replacement could silently compile tests with different flags.
        for configured in WORKFLOW['jobs']['build']['steps']:
            self.assertNotIn('CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS', configured.get('env', {}))

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
            self.assertIn(f'mascot-render-server: {OTHER_SHA}', notes)
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
            self.assertIn(f'clap-mml-render-tui: {OTHER_SHA}', notes)

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
            self.assertIn(f'cat-music-patterns: {OTHER_SHA}', notes)
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
            self.assertIn(f'clap-mml-play-server: {OTHER_SHA}', notes)
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


class ReleaseDiagnosticsTests(unittest.TestCase):
    def diagnostics(self, outcomes, dependencies=None):
        env = {'DIAGNOSTICS_DIR': str(Path('diagnostics').resolve()),
               'STEP_RESULTS': json.dumps({name: {'outcome': result} for name, result in outcomes.items()}),
               'DEPENDENCIES': json.dumps(dependencies or []), 'GITHUB_SHA': SHA,
               'RUN_URL': 'https://github.com/example/app/actions/runs/123',
               'GITHUB_STEP_SUMMARY': str(Path('summary').resolve())}
        with patch.dict(os.environ, env):
            exec(compile(step('build', 'Write build diagnostics'), '<diagnostics>', 'exec'), {})
        return Path('summary').read_text(encoding='utf-8')

    def test_failed_tests_keep_panic_cause_and_full_log(self):
        with workspace():
            Path('diagnostics').mkdir()
            # Successful tests containing "error" must not crowd the real failure out.
            log = ('test error_handling ... ok\n' * 300
                   + "thread 'variation' panicked at src/view/fixture.rs:10:5:\n"
                   + 'Default instrument missing: Surge XT.clap <missing>\n'
                   + 'test result: FAILED. 8 failed\n')
            Path('diagnostics/test.log').write_text(log, encoding='utf-8')
            summary = self.diagnostics({'project': 'success', 'test': 'failure', 'package': 'skipped'})
            self.assertIn('| test | failure |', summary)
            self.assertIn('src/view/fixture.rs:10:5', summary)
            self.assertIn('Default instrument missing: Surge XT.clap &lt;missing&gt;', summary)
            self.assertIn('test result: FAILED', summary)
            self.assertEqual(Path('diagnostics/test.log').read_text(encoding='utf-8'), log)

    def test_early_checkout_failure_still_writes_context(self):
        dependency = {'repository': 'example/shared', 'path': 'shared', 'sha': OTHER_SHA}
        with workspace(), patch('subprocess.run', return_value=subprocess.CompletedProcess([], 128, '', 'missing')):
            summary = self.diagnostics({'siblings': 'failure', 'project': 'skipped'}, [dependency])
            context = json.loads(Path('diagnostics/context.json').read_text(encoding='utf-8'))
            self.assertEqual(context['GITHUB_SHA'], SHA)
            self.assertEqual(context['dependencies'][0]['sha'], OTHER_SHA)
            self.assertIsNone(context['dependencies'][0]['checked_out_sha'])
            self.assertIn('| siblings | failure |', summary)

    def test_build_failure_keeps_cmake_evidence_and_nonstandard_error_tail(self):
        with workspace():
            Path('app/target/shim/CMakeFiles').mkdir(parents=True)
            Path('app/target/shim/CMakeCache.txt').write_text('generator evidence', encoding='utf-8')
            Path('app/target/shim/CMakeFiles/CMakeConfigureLog.yaml').write_text('configure evidence', encoding='utf-8')
            Path('diagnostics').mkdir()
            Path('diagnostics/build.log').write_text('custom adapter stopped\n', encoding='utf-8')
            summary = self.diagnostics({'project': 'failure', 'test': 'skipped'})
            self.assertIn('custom adapter stopped', summary)
            self.assertEqual(Path('diagnostics/CMakeCache.txt').read_text(), 'generator evidence')
            self.assertEqual(Path('diagnostics/CMakeConfigureLog.yaml').read_text(), 'configure evidence')

    @unittest.skipUnless(shutil.which('pwsh'), 'PowerShell 7 required')
    def test_build_wrapper_preserves_exit_and_terminating_errors(self):
        for script, expected in [("Write-Output 'build output'; exit 37", 37),
                                 ("throw 'adapter failure'", 1),
                                 ("Write-Output 'build succeeded'", 0)]:
            with self.subTest(script=script), workspace():
                Path('adapter.ps1').write_text(script, encoding='utf-8')
                Path('wrapper.ps1').write_text(
                    "$ErrorActionPreference = 'Stop'\n" + step('build', 'Project build and checks'),
                    encoding='utf-8')
                env = {**os.environ, 'DIAGNOSTICS_DIR': str(Path('diagnostics').resolve()),
                       'BUILD_SCRIPT': 'adapter.ps1'}
                result = subprocess.run(['pwsh', '-NoProfile', '-File', 'wrapper.ps1'],
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
                log = Path('diagnostics/build.log').read_text(encoding='utf-8-sig')
                self.assertIn('adapter failure' if expected == 1 else 'build', log)

    @unittest.skipUnless(shutil.which('pwsh'), 'PowerShell 7 required')
    def test_test_wrapper_preserves_native_exit_and_stderr(self):
        with workspace():
            # A child native process exercises LASTEXITCODE through Tee-Object.
            prelude = ("$ErrorActionPreference = 'Stop'\nfunction cargo { & pwsh -NoProfile -Command "
                       "\"[Console]::Error.WriteLine('native test failure'); exit 37\" }\n")
            Path('wrapper.ps1').write_text(prelude + step('build', 'Test'), encoding='utf-8')
            result = subprocess.run(['pwsh', '-NoProfile', '-File', 'wrapper.ps1'],
                                    env={**os.environ, 'DIAGNOSTICS_DIR': str(Path('diagnostics').resolve())},
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 37, result.stdout + result.stderr)
            self.assertIn('native test failure', Path('diagnostics/test.log').read_text(encoding='utf-8-sig'))

    def test_final_report_handles_all_failure_stages_and_unchanged_skip(self):
        for failed in ['resolve', 'build', 'publish', None]:
            with self.subTest(failed=failed), workspace():
                jobs = {name: {'result': 'failure' if name == failed else 'skipped', 'outputs': {}}
                        for name in ['resolve', 'build', 'publish']}
                if failed is None:
                    jobs['resolve'] = {'result': 'success', 'outputs': {'build': 'false'}}
                env = {'JOB_RESULTS': json.dumps(jobs), 'GITHUB_SHA': SHA, 'RUN_URL': 'https://example.com/run',
                       'GITHUB_STEP_SUMMARY': str(Path('summary').resolve())}
                with patch.dict(os.environ, env):
                    exec(compile(step('report', 'Release result'), '<report>', 'exec'), {})
                report = Path('summary').read_text(encoding='utf-8')
                self.assertIn(f'| {failed} | failure |' if failed else 'published inputs are unchanged', report)

    def test_diagnostics_run_after_failure_and_cannot_enter_publication_inputs(self):
        for name in ['Write build diagnostics', 'Upload build diagnostics']:
            configured = next(s for s in WORKFLOW['jobs']['build']['steps'] if s.get('name') == name)
            self.assertEqual(configured['if'], 'always()')
        self.assertEqual(WORKFLOW['jobs']['report']['if'], 'always()')
        downloads = [s['with']['name'] for s in WORKFLOW['jobs']['publish']['steps']
                     if s.get('uses', '').startswith('actions/download-artifact@')]
        self.assertEqual(downloads, ['windows-x64', 'release-notes'])


if __name__ == '__main__':
    unittest.main()
