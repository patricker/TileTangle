#!/usr/bin/env python3
"""Compare Criterion results against a stored baseline and print a summary."""
from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict


def load_results(results_dir: Path) -> Dict[str, float]:
  benchmarks: Dict[str, float] = {}
  if not results_dir.exists():
    raise SystemExit(f"results directory '{results_dir}' not found; run cargo bench first")
  for estimates in results_dir.glob("*/new/estimates.json"):
    try:
      data = json.loads(estimates.read_text())
    except json.JSONDecodeError as exc:
      raise SystemExit(f"failed to parse {estimates}: {exc}") from exc
    mean = data.get("mean", {}).get("point_estimate")
    if mean is None:
      raise SystemExit(f"missing mean.point_estimate in {estimates}")
    benchmarks[estimates.parent.parent.name] = float(mean)
  if not benchmarks:
    raise SystemExit(f"no benchmarks found under '{results_dir}'")
  return benchmarks


def write_baseline(path: Path, benchmarks: Dict[str, float]) -> None:
  payload = {
      "recorded_at": datetime.now(timezone.utc).isoformat(),
      "unit": "ns",
      "benchmarks": {name: benchmarks[name] for name in sorted(benchmarks)},
  }
  path.parent.mkdir(parents=True, exist_ok=True)
  path.write_text(json.dumps(payload, indent=2, sort_keys=False) + "\n")


def compare(baseline: Dict[str, float], current: Dict[str, float], threshold: float) -> int:
  header = f"{'benchmark':<28}{'baseline (ns)':>16}{'current (ns)':>16}{'delta (ns)':>14}{'ratio':>10}"
  print(header)
  print("-" * len(header))
  exit_code = 0
  for name in sorted(baseline):
    base = baseline[name]
    cur = current.get(name)
    if cur is None:
      print(f"{name:<28}{base:>16.2f}{'—':>16}{'n/a':>14}{'n/a':>10}")
      continue
    delta = cur - base
    ratio = cur / base if base else float("inf")
    print(f"{name:<28}{base:>16.2f}{cur:>16.2f}{delta:>14.2f}{ratio:>10.2f}x")
    if ratio >= threshold:
      exit_code = 1
  missing = sorted(name for name in current if name not in baseline)
  for name in missing:
    cur = current[name]
    print(f"{name:<28}{'—':>16}{cur:>16.2f}{'n/a':>14}{'n/a':>10}")
  if exit_code:
    print(f"\nDetected regression ≥ {threshold:.2f}× slower than baseline")
  return exit_code


def main() -> int:
  parser = argparse.ArgumentParser(description=__doc__)
  parser.add_argument("--results-dir", default="target/criterion", type=Path, help="Directory with Criterion outputs")
  parser.add_argument("--baseline", type=Path, required=True, help="Baseline JSON file")
  parser.add_argument("--write-baseline", action="store_true", help="Write a new baseline from the current results")
  parser.add_argument("--threshold", type=float, default=2.0, help="Slowdown ratio that causes a non-zero exit code")
  args = parser.parse_args()

  current = load_results(args.results_dir)
  if args.write_baseline:
    write_baseline(args.baseline, current)
    print(f"Baseline written to {args.baseline}")
    return 0
  try:
    baseline_payload = json.loads(args.baseline.read_text())
  except FileNotFoundError:
    raise SystemExit(f"baseline file '{args.baseline}' not found; run with --write-baseline first")
  except json.JSONDecodeError as exc:
    raise SystemExit(f"failed to parse baseline '{args.baseline}': {exc}") from exc
  if "benchmarks" not in baseline_payload:
    raise SystemExit(f"baseline file '{args.baseline}' missing 'benchmarks' object")
  baseline = {name: float(value) for name, value in baseline_payload["benchmarks"].items()}
  return compare(baseline, current, args.threshold)


if __name__ == "__main__":
  sys.exit(main())
