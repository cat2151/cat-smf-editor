"""Copy real Surge cache entries into isolated probe fixtures; never edit originals."""
import argparse
import json
from pathlib import Path
import struct


def fxp_chunk(path):
    data = path.read_bytes()
    if not (data[:4] == b"CcnK" and data[8:12] == b"FPCh" and data[16:20] == b"cjs3"):
        raise ValueError(f"Unsupported Surge FXP: {path}")
    chunk = data[60:]
    if len(chunk) < 32 or chunk[:4] != b"sub3":
        raise ValueError(f"Missing sub3: {path}")
    if len(chunk) != 32 + sum(struct.unpack("<7I", chunk[4:32])):
        raise ValueError(f"Invalid sub3 lengths: {path}")
    if struct.unpack(">I", data[56:60])[0] != len(chunk):
        raise ValueError(f"Invalid FXP chunk length: {path}")
    return list(chunk)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="New directory")
    parser.add_argument("--templates", type=Path, help="Original patches_factory/Templates")
    args = parser.parse_args()
    if args.output.resolve().is_relative_to(args.cache.resolve()):
        parser.error("Output must be outside the original cache")
    args.output.mkdir(parents=True, exist_ok=False)
    target = args.output / "input"
    target.mkdir()
    counts = dict(instruments=0, effects=0, raw_fxp=0)
    sample = None
    for path in sorted(args.cache.glob("*.json")):
        cached = json.loads(path.read_bytes())
        plugin = cached["reference"]["plugin"]["id"]
        if plugin == "org.surge-synth-team.surge-xt":
            counts["instruments"] += 1
            counts["raw_fxp"] += 1
            cached["fxp_state"] = fxp_chunk(Path(cached["reference"]["source"]["file"]))
            sample = cached
        elif plugin == "org.surge-synth-team.surge-xt-fx":
            counts["effects"] += 1
        else:
            continue
        (target / path.name).write_text(json.dumps(cached), encoding="utf8")
    if args.templates:
        if sample is None:
            raise ValueError("No Surge instrument fixture")
        templates = args.output / "templates"
        templates.mkdir()
        for name in ["Init Wavetable", "Init FM2", "Init Modern"]:
            cached = json.loads(json.dumps(sample))
            source = args.templates / f"{name}.fxp"
            cached["state"] = fxp_chunk(source)
            cached.pop("fxp_state", None)
            cached["reference"]["source"]["file"] = str(source)
            cached["reference"]["display"] = name
            (templates / f"{name}.json").write_text(json.dumps(cached), encoding="utf8")
    (args.output / "manifest.json").write_text(json.dumps(counts, indent=2), encoding="utf8")
    print(json.dumps(counts))


if __name__ == "__main__":
    main()
