"""Offline CLAP lifecycle probe. No audio device, GUI, notes, or plugin patches.

Set SURGE_DATA_HOME before running; this script does not edit the environment.
ABI follows uapmd/build/_deps/clap-src/include/clap/{entry,host,plugin}.h.
Run separate processes to distinguish first-instance work from subsequent instances.
"""
import argparse
import ctypes as c
import json
import os
from pathlib import Path
import time

P = c.c_void_p
FN = c.CFUNCTYPE


class Version(c.Structure):
    _fields_ = [("major", c.c_uint32), ("minor", c.c_uint32), ("revision", c.c_uint32)]


class Entry(c.Structure):
    _fields_ = [("version", Version), ("init", FN(c.c_bool, c.c_char_p)),
                ("deinit", FN(None)), ("get_factory", FN(P, c.c_char_p))]


EXT = FN(P, P, c.c_char_p)
REQUEST = FN(None, P)


class Host(c.Structure):
    _fields_ = [("version", Version), ("data", P), ("name", c.c_char_p),
                ("vendor", c.c_char_p), ("url", c.c_char_p), ("host_version", c.c_char_p),
                ("get_extension", EXT), ("restart", REQUEST),
                ("process", REQUEST), ("callback", REQUEST)]


class Factory(c.Structure):
    _fields_ = [("count", FN(c.c_uint32, P)), ("descriptor", FN(P, P, c.c_uint32)),
                ("create", FN(P, P, c.POINTER(Host), c.c_char_p))]


class Plugin(c.Structure):
    _fields_ = [("descriptor", P), ("data", P), ("init", FN(c.c_bool, P)),
                ("destroy", FN(None, P)),
                ("activate", FN(c.c_bool, P, c.c_double, c.c_uint32, c.c_uint32)),
                ("deactivate", FN(None, P)), ("start", P), ("stop", P),
                ("reset", P), ("process", P), ("get_extension", EXT),
                ("on_main_thread", FN(None, P))]


class IoCounters(c.Structure):
    _fields_ = [(name, c.c_uint64) for name in
                ("read_ops", "write_ops", "other_ops", "read_bytes", "write_bytes", "other_bytes")]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--instances", type=int, default=5)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if os.name != "nt" or c.sizeof(P) != 8 or args.instances < 1:
        parser.error("Requires 64-bit Windows and installed Surge XT CLAP")
    path = Path(os.environ["CommonProgramFiles"]) / "CLAP/Surge Synth Team/Surge XT.clap"
    events = []
    kernel = c.WinDLL("kernel32", use_last_error=True)
    kernel.GetCurrentProcess.restype = P
    kernel.GetProcessIoCounters.argtypes = [P, c.POINTER(IoCounters)]
    kernel.GetProcessIoCounters.restype = c.c_bool

    def io_counters():
        counters = IoCounters()
        if not kernel.GetProcessIoCounters(kernel.GetCurrentProcess(), c.byref(counters)):
            raise c.WinError(c.get_last_error())
        return {name: getattr(counters, name) for name, _ in counters._fields_}

    def measure(name, function, index=None):
        io_before = io_counters()
        wall, cpu = time.perf_counter_ns(), time.process_time_ns()
        result = function()
        event = dict(name=name, index=index, wall_ms=(time.perf_counter_ns()-wall)/1e6,
                     process_cpu_ms=(time.process_time_ns()-cpu)/1e6)
        event["io"] = {k: v-io_before[k] for k, v in io_counters().items()}
        events.append(event)
        return result

    library = measure("load_library", lambda: c.CDLL(str(path)))
    entry = Entry.in_dll(library, "clap_entry")
    if not measure("entry_init", lambda: entry.init(str(path).encode("utf8"))):
        raise RuntimeError("clap_entry.init failed")
    factory_address = measure("get_factory", lambda: entry.get_factory(b"clap.plugin-factory"))
    if not factory_address:
        raise RuntimeError("CLAP factory missing")
    factory = c.cast(factory_address, c.POINTER(Factory)).contents
    # Keep callbacks/host alive through destruction. No optional host extensions.
    extension = EXT(lambda *_: None)
    request = REQUEST(lambda *_: None)
    host = Host(Version(1, 2, 0), None, b"cat-smf lifecycle probe", b"cat-smf-editor",
                b"", b"1", extension, request, request, request)
    live = []
    try:
        for index in range(args.instances):
            address = measure("factory_create", lambda: factory.create(
                factory_address, c.byref(host), b"org.surge-synth-team.surge-xt"), index)
            if not address:
                raise RuntimeError("CLAP factory.create_plugin failed")
            plugin = c.cast(address, c.POINTER(Plugin)).contents
            live.append((address, plugin, False))
            if not measure("plugin_init", lambda: plugin.init(address), index):
                raise RuntimeError("CLAP plugin.init failed")
            if not measure("plugin_activate", lambda: plugin.activate(address, 48000, 1, 1024), index):
                raise RuntimeError("CLAP plugin.activate failed")
            live[-1] = (address, plugin, True)
            measure("main_thread_callback", lambda: plugin.on_main_thread(address), index)
    finally:
        for address, plugin, active in reversed(live):
            if active:
                plugin.deactivate(address)
            plugin.destroy(address)
        entry.deinit()
    result = dict(plugin=str(path), data_home=os.environ.get("SURGE_DATA_HOME"),
                  events=events, note="No audio device or process() calls; instances retained together")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2), encoding="utf8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
