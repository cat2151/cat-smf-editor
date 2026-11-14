"""Verify real SMF exports in fresh, silent repl sessions, including SFZ and DAGs.

Set UAPMD_TEST_PROJECT to an SMF containing an SFZ instrument and a track effect.
The second export adds a test-owned octave layer, bypassed effect and master effect
in memory. The source SMF and application settings are never saved by this test.
"""
import json
import math
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import struct
import tempfile
import time
import xml.etree.ElementTree as ET
import zipfile
import zlib

ROOT = Path(__file__).absolute().parents[1]
REPL = ROOT.parent / 'repl-plugin-player'


def aria_state(data):
    xml = ET.fromstring(zlib.decompress(data[8:]))
    # ARIA increments its save counter on every state save.
    for element in xml.iter():
        element.attrib.pop('sc', None)
    return ET.tostring(xml)


def verify(session, archive):
    session.evaluate('repl.project.load(' + json.dumps(str(archive)) + ');')
    tracks = session.evaluate('uapmd.sequencer.getTrackInfos();')
    sfz = effects = groups = 0
    with zipfile.ZipFile(archive) as files:
        project = json.loads(files.read('project.uapmd'))
        assert len(tracks) == len(project['tracks'])
        regular = list(zip(project['tracks'], [track['trackId'] for track in tracks]))
        master = session.evaluate('uapmd.timeline.getMasterTrackId();')
        for track, track_id in regular + [(project['master_track'], master)]:
            if 'graph' not in track:
                continue
            graph = json.loads(files.read(track['graph']['external_file']))
            restored = session.evaluate('uapmd.timeline.getTrackGraphConnections(' + json.dumps(track_id) + ');')
            assert restored['success'], restored
            actual = {(e['busType'], e['source']['nodeId'], e['target']['nodeId'])
                      for e in restored['connections']}
            # UAPMD inserts its track fader immediately before graph output.
            actual = {(bus, source, 'graph:output' if target == 'builtin:track_gain' else target)
                      for bus, source, target in actual if source != 'builtin:track_gain'}
            expected = {(e['bus_type'], e['source']['node_id'], e['target']['node_id'])
                        for e in graph['connections']}
            assert actual == expected, (track_id, expected - actual, actual - expected,
                                       graph['plugins'], tracks)
            for plugin in graph['plugins']:
                node_id = plugin['node_id']
                # A bypassed effect is intentionally disconnected, but still restored.
                node = next((n for t in tracks for n in t['nodes']
                             if t['trackId'] == track_id and n['nodeId'] == node_id), None)
                if node is None:
                    # The master is not included in getTrackInfos(). Query its edges;
                    # the fixture's master effect is active and therefore connected.
                    endpoint = next((e[side] for e in restored['connections'] for side in ('source', 'target')
                                     if e[side]['nodeId'] == node_id), None)
                    assert endpoint is not None, (track_id, node_id)
                    instance = endpoint['instanceId']
                else:
                    assert node['pluginId'] == plugin['plugin_id'], node
                    instance = node['instanceId']
                if plugin['state_file']:
                    original = files.read(plugin['state_file'])
                    state = bytes.fromhex(session.evaluate(f'repl.test.pluginState({instance});'))
                    if 'sforzando' in plugin['plugin_id']:
                        assert aria_state(original) == aria_state(state), 'SFZ program changed'
                        sfz += 1
                    elif node_id.startswith('effect:'):
                        assert original == state, (track_id, node_id, 'plugin state changed')
                    else:
                        assert state, (track_id, node_id, 'empty plugin state')
                effects += node_id.startswith('effect:')
            for clip in track['clips']:
                if clip['file'].endswith('.midi2'):
                    clips = session.evaluate('uapmd.timeline.getClips(' + json.dumps(track_id) + ');')
                    events = session.evaluate('uapmd.timeline.getClipUmpEvents(' + json.dumps(clips[0]['clipId']) + ');')['events']
                    groups = max(groups, len({(e['words'][0] >> 24) & 15 for e in events
                                              if e['words'][0] >> 28 in (2, 4)}))
    return sfz, effects, groups


def sfz_audio(session, archive, work, group=0):
    """Freeze a short test-owned drum clip: exercise actual repl audio without a device."""
    with zipfile.ZipFile(archive) as source:
        plugins = [plugin for name in source.namelist() if name.startswith('graphs/')
                   for plugin in json.loads(source.read(name))['plugins']]
        plugin = dict(next(p for p in plugins if 'sforzando' in p['plugin_id']))
        state = source.read(plugin['state_file'])
    plugin.update(node_id='plugin:0', group_index=group, state_file='plugin_states/sfz.state')
    endpoint = lambda kind, name: dict(type=kind, node_id=name, bus_index=0)
    graph = dict(graph_type='urn:uapmd-graph:common/graph/dag/v1', plugins=[plugin], connections=[
        dict(id=1, bus_type='event', source=endpoint('graph_input', 'graph:input'), target=endpoint('plugin', 'plugin:0')),
        dict(id=2, bus_type='audio', source=endpoint('plugin', 'plugin:0'), target=endpoint('graph_output', 'graph:output'))])
    track = bytes([0, 0x90, 36, 100, 96, 0x80, 36, 0, 0, 0xff, 0x2f, 0])
    midi = b'MThd' + struct.pack('>IHHH', 6, 0, 1, 480) + b'MTrk' + struct.pack('>I', len(track)) + track
    if group:
        words = [0x00400000, 0x003001e0, 0x00400000, 0xf0200000, 0, 0, 0,
                 0x00400000, 0x20902464 | group << 24, 0x00400060, 0x20802400 | group << 24,
                 0x00400000, 0xf0210000, 0, 0, 0]
        midi = b'SMF2CLIP' + struct.pack(f'>{len(words)}I', *words)
    clip_file = 'clips/drum.midi2' if group else 'clips/drum.mid'
    project = dict(master_track=dict(volume=1, clips=[]), tracks=[dict(volume=1,
        graph=dict(graph_type=graph['graph_type'], external_file='graphs/sfz.json'),
        clips=[dict(position_samples=0, file=clip_file, clip_type='midi', tick_resolution=480)])])
    path = work / 'sfz-audio.uapmdz'
    with zipfile.ZipFile(path, 'w') as output:
        for name, data in [('project.uapmd', json.dumps(project).encode()),
                           ('graphs/sfz.json', json.dumps(graph).encode()),
                           (clip_file, midi), ('plugin_states/sfz.state', state)]:
            output.writestr(name, data)
    session.evaluate('repl.project.load(' + json.dumps(str(path)) + ');')
    track_id = session.evaluate('uapmd.sequencer.getTrackInfos()[0].trackId;')
    before = set(work.rglob('*.frozen'))
    session.evaluate('uapmd.sequencer.setTrackFreezePolicy(' + json.dumps(track_id) + ', "on");')
    deadline = time.monotonic() + 60
    while True:
        status = session.evaluate('uapmd.sequencer.getTrackFreezeState(' + json.dumps(track_id) + ');')
        assert status['runtimeState'] != 'error', status
        if status['runtimeState'] == 'frozen':
            break
        assert time.monotonic() < deadline, status
        time.sleep(0.05)
    frozen = work / 'sfz-frozen.uapmdz'
    session.evaluate('repl.project.saveArchive(' + json.dumps(str(frozen)) + ');')
    session.evaluate('uapmd.sequencer.setTrackFreezePolicy(' + json.dumps(track_id) + ', "off");')
    while session.evaluate('uapmd.sequencer.getTrackFreezeState(' + json.dumps(track_id) + ');')['runtimeState'] != 'live':
        assert time.monotonic() < deadline
        time.sleep(0.05)
    session.evaluate('repl.project.load(' + json.dumps(str(archive)) + ');')
    audio = [path.read_bytes() for path in work.rglob('*.frozen') if path not in before]
    assert audio, 'Missing frozen audio in isolated application data'
    peak = 0.0
    for data in audio:
        end = struct.unpack_from('<Q', data, len(data) - 20)[0]
        offset = 0
        while offset < end:
            _, literals = struct.unpack_from('<II', data, offset)
            offset += 8
            if literals:
                values = struct.unpack_from(f'<{literals}f', data, offset)
                assert all(math.isfinite(value) for value in values), 'Nonfinite SFZ audio'
                peak = max(peak, max(abs(value) for value in values))
            offset += literals * 4
        assert offset == end
    assert peak > 1e-4, f'SFZ rendered silence: peak={peak}'
    print(f'PASS: real repl SFZ group={group} drum peak={peak:.6f}; no audio device playback')


def main():
    source = Path(os.environ['UAPMD_TEST_PROJECT'])
    local = Path(os.environ['LOCALAPPDATA'])
    session_type = runpy.run_path(str(REPL / 'scripts/cli-test.py'))['Session']
    with tempfile.TemporaryDirectory(prefix='smfe-uapmd-effects-') as temporary:
        work = Path(temporary)
        isolated = work / 'local'
        cache = isolated / 'repl-plugin-player/uapmd'
        cache.mkdir(parents=True)
        shutil.copy2(local / 'repl-plugin-player/uapmd/plugin-list-cache.json', cache)

        def launch(*args, **kwargs):
            return subprocess.Popen([str(REPL / 'target/release/repl-plugin-player.exe'),
                                     '--lang', 'en', *args], cwd=REPL,
                                    env=dict(os.environ, LOCALAPPDATA=str(isolated)),
                                    text=True, encoding='utf-8',
                                    creationflags=subprocess.CREATE_NO_WINDOW, **kwargs)

        session_type.__init__.__globals__['launch'] = launch
        for layered in (False, True):
            output = work / f'export-{layered}.uapmdz'
            env = dict(os.environ, UAPMD_TEST_PROJECT=str(source), UAPMD_TEST_EXPORT=str(output))
            env.pop('UAPMD_TEST_LAYERED', None)
            if layered:
                env['UAPMD_TEST_LAYERED'] = '1'
            subprocess.run(['cargo', 'test', '--release', '--bin', 'cat-smf-editor',
                            'real_project_export_fixture', '--', '--ignored', '--test-threads=1'],
                           cwd=ROOT, env=env, check=True)
            session = session_type()
            try:
                counts = verify(session, output)
                assert counts[0] > 0 and counts[1] > 0, counts
                if layered:
                    assert counts[2] == 2, counts
                else:
                    sfz_audio(session, output, work)
                    sfz_audio(session, output, work, group=1)
                session.finish()
            finally:
                session.kill()
            print(f'PASS: layered={layered}, SFZ={counts[0]}, effects={counts[1]}, groups={counts[2]}')


if __name__ == '__main__':
    main()
