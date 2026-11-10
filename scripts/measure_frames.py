"""Measures playback frames of the release editor on a copy of the user's data.

python scripts/measure_frames.py drum|plain|saved [no-keyboard] [idle]

`drum` measures with the Drum sequencer open, `plain` without it, `saved` with the track
and position the user's session left. `no-keyboard` measures
without the Virtual MIDI keyboard. `idle` first waits until the startup playback has ended
and every audio stream rests, then measures the editor's CPU per thread and its frames for
`IDLE_SECONDS` before playing; it needs psutil. The editor runs with a temporary LOCALAPPDATA and
TEMP; the real song's hash is compared afterwards. The editor's windows take the
foreground while it runs.
"""
import ctypes
import ctypes.wintypes as wintypes
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
EDITOR = REPO / "target" / "release" / "cat-smf-editor.exe"
REPORT = REPO / "scripts" / "frame_report.py"

SONG_WINDOW = "Pattern / Song"
# A dock window's title lists its panes.
SONG_PANE = "· song"
KEYBOARD_PANE = "· keyboard"
DRUM_SPAN = "contents/drums/sounding"
KEYBOARD_SPAN = "keyboard/ui"
STARTUP_TIMEOUT = 60.0
SOUNDS_READY = 12.0
PLAY_SECONDS = 6.0
CLOSE_TIMEOUT = 8.0
DRUM_TRACK_NAME = b"\xff\x03\x05Drums"
REST_TIMEOUT = 90.0
IDLE_SECONDS = 6.0
HOT_CORE = 0.8

WM_CLOSE = 0x0010
WM_KEYDOWN = 0x0100
WM_KEYUP = 0x0101
VK_SPACE = 0x20
SPACE_SCAN = 0x39

user32 = ctypes.WinDLL("user32", use_last_error=True)
user32.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
user32.SetForegroundWindow.argtypes = [wintypes.HWND]
ENUM_WINDOWS = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)


class MeasureError(Exception):
    pass


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path, value):
    path.write_text(json.dumps(value), encoding="utf-8")


def same_file(left, right):
    plain = [os.path.normcase(str(path).removeprefix("\\\\?\\")) for path in (left, right)]
    return plain[0] == plain[1]


def session_track(song, mode):
    """The 1-based track to restore; selecting a drum track opens the Drum sequencer."""
    data = song.read_bytes()
    chunks = []
    start = data.find(b"MTrk")
    while start != -1:
        size = int.from_bytes(data[start + 4 : start + 8], "big")
        chunks.append(data[start + 8 : start + 8 + size])
        start = data.find(b"MTrk", start + 8 + size)
    drums = [DRUM_TRACK_NAME in chunk for chunk in chunks]
    if True not in drums:
        raise MeasureError(f"{song} has no track named Drums")
    # The first chunk is the conductor track.
    return drums.index(mode == "drum", 1) + 1


def isolate(work, mode, keyboard):
    """Returns the real song, the copied song and the copied data directory."""
    source = Path(os.environ["LOCALAPPDATA"]) / "cat-smf-editor"
    data = work / "local" / "cat-smf-editor"
    shutil.copytree(source, data, ignore=shutil.ignore_patterns("backup", "*.log"))
    song = Path(read_json(source / "startup.json")["last_mid"])
    copy = work / song.name
    shutil.copy(song, copy)
    # startup.json wins over status.json when the editor chooses the document.
    write_json(data / "startup.json", {"last_mid": str(copy)})
    status = read_json(data / "status.json")
    status["navigation"]["last_mid"] = str(copy)
    if mode != "saved":
        status["session"] = {
            "file": song.name,
            "track": session_track(song, mode),
            "mbt": "1:1:0",
            "tick": 0,
            "row": 0,
            "note_mode": True,
        }
    if not keyboard:
        # The Song window reopens a closed keyboard; only a failed opening keeps it shut.
        status["keyboard_settings"][0]["tempo"] = 0
    write_json(data / "status.json", status)
    return song, copy, data


def launch(work, trace):
    temp = work / "temp"
    temp.mkdir()
    environment = dict(
        os.environ,
        LOCALAPPDATA=str(work / "local"),
        TEMP=str(temp),
        TMP=str(temp),
        CAT_SMF_FRAME_TRACE=str(trace),
    )
    return subprocess.Popen(
        [EDITOR],
        env=environment,
        cwd=REPO,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def windows(process):
    """Visible top-level windows of the process, title to handle."""
    found = {}

    def visit(hwnd, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value == process.pid and user32.IsWindowVisible(hwnd):
            title = ctypes.create_unicode_buffer(256)
            user32.GetWindowTextW(hwnd, title, 256)
            found[title.value] = hwnd
        return True

    user32.EnumWindows(ENUM_WINDOWS(visit), 0)
    return found


def window_threads(process):
    threads = set()

    def visit(hwnd, _):
        owner = wintypes.DWORD()
        thread = user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value == process.pid:
            threads.add(thread)
        return True

    user32.EnumWindows(ENUM_WINDOWS(visit), 0)
    return threads


def wait_for_rest(process, log, streams):
    """Waits until `streams` audio streams rest after the editor's last audio command."""
    pattern = re.compile(rf"\[audio-rest\] pid={process.pid} stream=(\d+)")
    deadline = time.monotonic() + REST_TIMEOUT
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise MeasureError(f"editor exited before resting: {process.returncode}")
        lines = log.read_text(encoding="utf-8", errors="replace").splitlines() if log.exists() else []
        commands = [i for i, line in enumerate(lines) if "[audio-command]" in line]
        if commands:
            rested = {m.group(1) for line in lines[commands[-1] + 1 :] if (m := pattern.search(line))}
            if len(rested) >= streams:
                return
        time.sleep(0.2)
    raise MeasureError(f"fewer than {streams} audio streams rested within {REST_TIMEOUT} s")


def last_frame(trace):
    """The latest frame the editor has flushed; it flushes about once a second."""
    lines = trace.read_text(encoding="utf-8").splitlines() if trace.exists() else []
    for line in reversed(lines):
        try:
            return json.loads(line)
        except ValueError:
            continue
    raise MeasureError("the frame trace is empty")


def measure_idle(process, trace):
    """Prints the editor's CPU and its busiest threads; returns the idle frames' range."""
    import psutil

    editor = psutil.Process(process.pid)
    ui = window_threads(process)
    first = last_frame(trace)
    started = time.perf_counter()
    before = editor.cpu_times()
    threads_before = {t.id: t.user_time + t.system_time for t in editor.threads()}
    time.sleep(IDLE_SECONDS)
    seconds = time.perf_counter() - started
    after = editor.cpu_times()
    threads_after = {t.id: t.user_time + t.system_time for t in editor.threads()}
    last = last_frame(trace)
    cores = (after.user + after.system - before.user - before.system) / seconds
    busy = sorted(
        ((used - threads_before.get(tid, 0)) / seconds, tid) for tid, used in threads_after.items()
    )
    print(f"\n== idle: {seconds:.1f} s")
    print(f"editor CPU          {cores / psutil.cpu_count() * 100:8.1f} %  ({cores:.2f} core)")
    print(f"threads over {HOT_CORE} core {sum(core > HOT_CORE for core, _ in busy):5}")
    for core, tid in reversed(busy[-3:]):
        print(f"  thread {tid:<6}{' UI' if tid in ui else '':4}{core:6.2f} core")
    return first["t_us"], last["t_us"]


def report_range(trace, work, name, keep):
    frames = [line for line in trace.read_text(encoding="utf-8").splitlines() if keep(json.loads(line))]
    part = work / name.replace(" ", "-")
    part.write_text("\n".join(frames) + "\n", encoding="utf-8")
    print(f"\n#### {name}", flush=True)
    subprocess.run([sys.executable, REPORT, part], check=True)


def main_window(process):
    return next(hwnd for title, hwnd in windows(process).items() if "cat-smf-editor" in title)


def song_window(process):
    """The Song window, floating or as a pane of a dock window."""
    found = windows(process)
    if SONG_WINDOW in found:
        return found[SONG_WINDOW]
    return next((hwnd for title, hwnd in found.items() if SONG_PANE in title), None)


def check_trace(trace, mode, keyboard, docked):
    """A run that shows the wrong windows or never played measures nothing. `docked` is
    whether a dock window's title listed the keyboard pane, which has no span."""
    text = trace.read_text(encoding="utf-8")
    if mode != "saved" and (DRUM_SPAN in text) != (mode == "drum"):
        raise MeasureError(f"{DRUM_SPAN} in a {mode} run: {DRUM_SPAN in text}")
    if (KEYBOARD_SPAN in text or docked) != keyboard:
        raise MeasureError(f"keyboard shown: {KEYBOARD_SPAN in text or docked}")
    if not any(json.loads(line)["tick"] is not None for line in text.splitlines()):
        raise MeasureError("the editor did not play")


def wait_for_copy(process, data, copy):
    """Keys may reach the editor only once it is known to have opened the copy."""
    recent = data / "recent-files.json"
    deadline = time.monotonic() + STARTUP_TIMEOUT
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise MeasureError(f"editor exited during startup: {process.returncode}")
        if song_window(process):
            entries = read_json(recent)
            if entries and same_file(entries[0]["path"], copy):
                return
        time.sleep(0.2)
    raise MeasureError(f"the first entry of {recent} is not {copy}")


def play(process):
    song = song_window(process)
    user32.SetForegroundWindow(song)
    time.sleep(0.5)
    flags = 1 | SPACE_SCAN << 16
    user32.PostMessageW(song, WM_KEYDOWN, VK_SPACE, flags)
    time.sleep(0.05)
    user32.PostMessageW(song, WM_KEYUP, VK_SPACE, flags | 0xC0000000)


def close(process):
    """Stops only the process this script started, with whatever it spawned."""
    if process.poll() is None:
        try:
            user32.PostMessageW(main_window(process), WM_CLOSE, 0, 0)
            process.wait(CLOSE_TIMEOUT)
        except (StopIteration, subprocess.TimeoutExpired):
            print("the editor did not close: killing it", flush=True)
            subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], capture_output=True)
            process.wait(CLOSE_TIMEOUT)


def measure(mode, keyboard, idle, work):
    trace = work / "frames.jsonl"
    song, copy, data = isolate(work, mode, keyboard)
    before = sha256(song)
    process = launch(work, trace)
    idle_range = None
    try:
        wait_for_copy(process, data, copy)
        docked = any(KEYBOARD_PANE in title for title in windows(process))
        if idle:
            # The song's stream, and the keyboard's when it is open.
            wait_for_rest(process, data / "error.log", 2 if keyboard else 1)
            time.sleep(1.0)
            idle_range = measure_idle(process, trace)
        else:
            time.sleep(SOUNDS_READY)
        play(process)
        time.sleep(PLAY_SECONDS)
    finally:
        close(process)
    if sha256(song) != before:
        raise MeasureError(f"the real song changed: {song}")
    check_trace(trace, mode, keyboard, docked)
    if idle_range:
        start, end = idle_range
        report_range(trace, work, "idle frames", lambda f: start <= f["t_us"] <= end)
        report_range(trace, work, "frames after idle", lambda f: f["t_us"] > end)
    else:
        subprocess.run([sys.executable, REPORT, trace], check=True)


def main():
    arguments = sys.argv[1:]
    idle = arguments[-1:] == ["idle"]
    if idle:
        arguments = arguments[:-1]
    if arguments[:1] not in (["drum"], ["plain"], ["saved"]) or arguments[1:] not in ([], ["no-keyboard"]):
        sys.exit(__doc__)
    sys.stdout.reconfigure(encoding="utf-8")
    work = Path(tempfile.mkdtemp(prefix="cat-smf-frames-"))
    try:
        measure(arguments[0], len(arguments) == 1, idle, work)
    except MeasureError as error:
        sys.exit(str(error))
    finally:
        shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
