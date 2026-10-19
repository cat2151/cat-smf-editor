"""Compare native_audio probe runs, keeping sound data separate from path metadata."""
import argparse
import array
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET


def state_content(data):
    if not data.startswith(b"sub3"):
        return data
    size = int.from_bytes(data[4:8], "little")
    root = ET.fromstring(data[32:32+size])
    for element in root.iter("lastLoadedPath"):
        if "v" in element.attrib:
            # Only this UI path is normalized, and only by resolving its actual target.
            element.set("v", str(Path(element.get("v")).resolve()))
    return data[8:32], ET.tostring(root), data[32+size:]


def rms(data):
    samples = array.array("f", data)
    return (sum(float(x)*x for x in samples)/len(samples))**0.5


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--baseline-recheck", type=Path,
                        help="An independent baseline process, to detect process-dependent phase")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = dict(cases=0, equal_state=0, state_differences=[], deterministic_cases=0,
                  deterministic_audio_equal=0, deterministic_audio_differences=[],
                  variable_audio=[])
    for first in sorted(args.baseline.glob("*-0.f32")):
        name = first.name[:-6]
        candidate = args.candidate / first.name
        second = args.baseline / first.name.replace("-0.f32", "-1.f32")
        if not candidate.exists() or not second.exists():
            raise RuntimeError(f"Incomplete probe: {name}")
        result["cases"] += 1
        state = first.with_suffix(".state").read_bytes()
        changed = candidate.with_suffix(".state").read_bytes()
        second_candidate = candidate.with_name(candidate.name.replace("-0.f32", "-1.f32"))
        if not second_candidate.exists():
            raise RuntimeError(f"Incomplete candidate repeat: {name}")
        changed_again = second_candidate.with_suffix(".state").read_bytes()
        if state_content(state) == state_content(changed) == state_content(changed_again):
            result["equal_state"] += 1
        else:
            result["state_differences"].append(name)
        a, b, c = first.read_bytes(), second.read_bytes(), candidate.read_bytes()
        another = (args.baseline_recheck / first.name).read_bytes() if args.baseline_recheck else a
        candidate_again = second_candidate.read_bytes()
        if a == b == another:
            result["deterministic_cases"] += 1
            if a == c == candidate_again:
                result["deterministic_audio_equal"] += 1
            else:
                result["deterministic_audio_differences"].append(name)
        else:
            result["variable_audio"].append(dict(name=name, baseline_rms=[rms(a), rms(b), rms(another)],
                                                 candidate_rms=[rms(c), rms(candidate_again)], sha256=hashlib.sha256(c).hexdigest()))
    args.output.write_text(json.dumps(result, indent=2), encoding="utf8")
    print(json.dumps({k:v for k,v in result.items() if k!="variable_audio"}, indent=2))


if __name__ == "__main__":
    main()
