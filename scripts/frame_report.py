"""Summarizes a CAT_SMF_FRAME_TRACE file: python scripts/frame_report.py <file>

Frames with a playing position and frames without one are reported separately.
"repaint requested by" counts, per second, the frames whose previous pass asked egui for
a repaint from that `file:line`, including requests with a delay.
"""
import json
import re
import statistics
import sys
from collections import defaultdict


def percentile(values, ratio):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(len(ordered) * ratio))]


def attach_next(frames):
    """Adds what only the next line knows; a discarded pass's time joins the painted frame."""
    carried = 0
    for frame, following in zip(frames, frames[1:]):
        paint = max(0, following["previous_cpu_us"] - frame["update_us"])
        frame["root_paint_us"] = min(paint, following["gap_us"])
        frame["root_swap_wait_us"] = following["gap_us"] - frame["root_paint_us"]
        frame["cpu_us"] = following["thread_cpu_us"] - frame["thread_cpu_us"]
        frame["moved"] = following["tick"] not in (None, frame["tick"])
        carried += following["t_us"] - frame["t_us"]
        frame["interval_us"] = 0 if frame["discarded"] else carried
        carried = 0 if not frame["discarded"] else carried
    return frames[:-1]


def parts(frame):
    """Exclusive times of one update. `x/(rest)` is the time in x outside its named parts,
    which for a window is its tessellate, paint and swap."""
    spans = defaultdict(int)
    for name, took in frame["spans"]:
        spans[name] += took
    result = {}
    for name, took in spans.items():
        children = sum(
            value
            for child, value in spans.items()
            if child.startswith(name + "/") and "/" not in child[len(name) + 1 :]
        )
        result[f"{name}/(rest)" if children else name] = took - children
    top = sum(took for name, took in spans.items() if "/" not in name)
    result["(other update)"] = max(0, frame["update_us"] - top)
    result["(root paint)"] = frame["root_paint_us"]
    result["(root swap or wait)"] = frame["root_swap_wait_us"]
    return result


def report(title, frames):
    painted = [f for f in frames if not f["discarded"]]
    seconds = sum(f["interval_us"] for f in frames) / 1e6
    if not painted or not seconds:
        print(f"\n== {title}: no frames")
        return
    intervals = [f["interval_us"] / 1000 for f in painted]
    print(f"\n== {title}: {seconds:.1f} s")
    print(f"frames/s            {len(painted) / seconds:8.1f}")
    if any(f["tick"] is not None for f in painted):
        print(f"position moves/s    {sum(f['moved'] for f in frames) / seconds:8.1f}")
    print(
        "frame interval ms   "
        f"median {statistics.median(intervals):.1f}  "
        f"p95 {percentile(intervals, 0.95):.1f}  max {max(intervals):.1f}"
    )
    print(f"UI thread CPU       {sum(f['cpu_us'] for f in frames) / seconds / 1e4:8.1f} %")
    print(f"discarded passes/s  {(len(frames) - len(painted)) / seconds:8.1f}")

    totals = defaultdict(int)
    for frame in frames:
        for name, took in parts(frame).items():
            totals[name] += took
    print(f"\n{'part':28}{'ms/frame':>10}{'share':>8}")
    whole = sum(totals.values()) or 1
    for name, took in sorted(totals.items(), key=lambda item: -item[1]):
        if took / whole >= 0.001:
            print(
                f"{name:28}{took / len(painted) / 1000:10.2f}{took / whole * 100:7.1f}%"
            )

    causes = defaultdict(int)
    for frame in painted:
        for cause in frame.get("causes", []):
            # A dependency's cause reads `egui-0.31.1\src\context.rs:3595`.
            causes[re.sub(r".*[\\/]registry[\\/]src[\\/][^\\/]+[\\/]", "", cause)] += 1
    if causes:
        print(f"\n{'repaint requested by':48}{'/s':>8}")
        for cause, count in sorted(causes.items(), key=lambda item: -item[1]):
            print(f"{cause:48}{count / seconds:8.1f}")


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    with open(sys.argv[1], encoding="utf-8") as file:
        frames = attach_next([json.loads(line) for line in file])
    report("playing", [f for f in frames if f["tick"] is not None])
    report("not playing", [f for f in frames if f["tick"] is None])


if __name__ == "__main__":
    main()
