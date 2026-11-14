"""Compare editor export with its source in fresh, silent repl-plugin-player sessions.

Requires built release binaries and the real note/patch import fixture. All outputs
and repl settings stay in a temporary directory outside the repositories.
"""
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
REPL = ROOT.parent / 'repl-plugin-player'
SNAPSHOT = r"""(() => {
    const tracks = uapmd.sequencer.getTrackInfos();
    return tracks.map((track, i) => ({
        volume:repl.track.volume(i),
        plugin:repl.test.pluginState(include('builtin/lib/plugins.js').instrumentOn(i)),
        clips:uapmd.timeline.getClips(track.trackId).map(clip => {
            const midi = uapmd.timeline.getClipUmpEvents(clip.clipId);
            return {position:clip.positionSamples, division:midi.tickResolution,
                events:midi.events.filter(e => [2,4].includes(e.words[0] >>> 28)).map(e => {
                    const word = e.words[0];
                    return [e.tick, (word >>> 16) & 255, (word >>> 8) & 127,
                        (word >>> 28) === 4 ? e.words[1] >>> 25 : word & 127];
                })};
        })
    }));
})();"""


def main():
    local = Path(os.environ['LOCALAPPDATA'])
    source = Path(os.environ.get('UAPMD_TEST_PROJECT', local / 'repl-plugin-player/projects/untitled.uapmdz'))
    with tempfile.TemporaryDirectory(prefix='smfe-uapmd-export-') as temporary:
        work = Path(temporary)
        output = work / 'export.uapmdz'
        subprocess.run(['cargo', 'test', '--release', '--bin', 'cat-smf-editor',
                        'real_project_export_fixture', '--', '--ignored', '--test-threads=1'],
                       cwd=ROOT, env=dict(os.environ, UAPMD_TEST_PROJECT=str(source),
                                          UAPMD_TEST_EXPORT=str(output)), check=True)
        session_type = runpy.run_path(str(REPL / 'scripts/cli-test.py'))['Session']
        isolated = work / 'local'
        cache = local / 'repl-plugin-player/uapmd/plugin-list-cache.json'
        if cache.is_file():
            destination = isolated / 'repl-plugin-player/uapmd'
            destination.mkdir(parents=True)
            shutil.copy2(cache, destination / cache.name)

        def launch(*args, **kwargs):
            return subprocess.Popen([str(REPL / 'target/release/repl-plugin-player.exe'),
                                     '--lang', 'en', *args], cwd=REPL,
                                    env=dict(os.environ, LOCALAPPDATA=str(isolated)),
                                    text=True, encoding='utf-8',
                                    creationflags=subprocess.CREATE_NO_WINDOW, **kwargs)

        session_type.__init__.__globals__['launch'] = launch
        snapshots = []
        for path in (source, output):
            session = session_type()
            try:
                session.evaluate('repl.project.load(' + json.dumps(str(path)) + ');')
                snapshots.append(session.evaluate(SNAPSHOT))
                session.finish()
            finally:
                session.kill()
        assert snapshots[0] == snapshots[1], 'Export changed note timing, MIDI values, volumes or plugin states'
        print('PASS: fresh repl loads preserve notes, timing, volume and plugin state; no audio playback')


if __name__ == '__main__':
    main()
