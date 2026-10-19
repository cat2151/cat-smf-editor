"""Diagnostic resource omission, NOT a playable configuration or app optimization.

Creates new temporary trees with junctions; never edits/removes original resources.
Each run is a fresh offline process using probe_surge_creation.py. No sound is rendered.
"""
import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys

VARIANTS = {
    "control": set(),
    "no_fx": {"fx_presets"},
    "no_wavetables": {"wavetables", "wavetables_3rdparty"},
    "no_wt_factory": {"wavetables"},
    "no_wt_thirdparty": {"wavetables_3rdparty"},
    "no_modulator": {"modulator_presets"},
    "no_skins": {"skins"},
    "no_tuning": {"tuning_library"},
    "no_fx_or_wt": {"fx_presets", "wavetables", "wavetables_3rdparty"},
    "control_after": set(),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-home", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="New directory; must not exist")
    parser.add_argument("--runs", type=int, default=3)
    args = parser.parse_args()
    if os.name != "nt" or args.runs < 1:
        parser.error("Requires Windows and at least one run")
    source = args.data_home.resolve(strict=True)
    if args.output.resolve().is_relative_to(source):
        parser.error("Output must be outside the source data tree")
    if not (source / "patches_factory/Templates").is_dir():
        parser.error("Source must preserve the initial Templates")
    args.output.mkdir(parents=True, exist_ok=False)
    summary = {}
    for label, omit in VARIANTS.items():
        home = args.output / label
        home.mkdir()
        for entry in source.iterdir():
            target = home / entry.name
            if entry.name in omit:
                target.mkdir()
            elif entry.is_dir():
                # Static command only; path values are expanded once inside quotes.
                subprocess.run('cmd /d /v:off /c mklink /J "%CAT_SMF_PROBE_LINK%" "%CAT_SMF_PROBE_SOURCE%"',
                               env=dict(os.environ, CAT_SMF_PROBE_LINK=str(target.resolve()),
                                        CAT_SMF_PROBE_SOURCE=str(entry.resolve())),
                               capture_output=True, creationflags=0x08000000, check=True)
            else:
                target.write_bytes(entry.read_bytes())
        runs = []
        for index in range(1, args.runs + 1):
            output = args.output / f"{label}-{index}.json"
            result = subprocess.run([sys.executable, str(Path(__file__).with_name("probe_surge_creation.py")),
                                     "--output", str(output.resolve())],
                                    env=dict(os.environ, SURGE_DATA_HOME=str(home.resolve())),
                                    capture_output=True, text=True, timeout=45)
            (args.output / f"{label}-{index}.log").write_text(result.stdout + result.stderr, encoding="utf8")
            result.check_returncode()
            events = json.loads(output.read_text(encoding="utf8"))["events"]
            events = [e for e in events if e["name"] == "factory_create"]
            runs.append(dict(wall_ms=sum(e["wall_ms"] for e in events),
                             cpu_ms=sum(e["process_cpu_ms"] for e in events),
                             read_ops=sum(e["io"]["read_ops"] for e in events),
                             other_ops=sum(e["io"]["other_ops"] for e in events)))
        summary[label] = dict(median={k: statistics.median(r[k] for r in runs) for k in runs[0]}, runs=runs)
        print(label, json.dumps(summary[label]["median"]), flush=True)
        (args.output / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf8")


if __name__ == "__main__":
    main()
