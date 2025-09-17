#!/usr/bin/env python3
"""Build distributable artifacts for TileTangle across supported bindings."""
from __future__ import annotations

import argparse
import json
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover
    import tomli as tomllib  # type: ignore


ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / "dist"


def run(cmd: list[str], *, cwd: Path | None = None, env: dict[str, str] | None = None) -> None:
    print("→", " ".join(cmd))
    subprocess.run(cmd, cwd=cwd or ROOT, env=env, check=True)


def read_version() -> str:
    data = tomllib.loads((ROOT / "engine" / "Cargo.toml").read_text())
    return data["package"]["version"]


def clean_dist() -> None:
    if DIST.exists():
        shutil.rmtree(DIST)
    DIST.mkdir(parents=True, exist_ok=True)


def package_rust(version: str) -> None:
    run(["cargo", "package", "-p", "tiletangle-engine", "--allow-dirty", "--no-verify"])
    src = ROOT / "target" / "package" / f"tiletangle-engine-{version}.crate"
    if not src.exists():
        raise FileNotFoundError(f"expected crate artifact at {src}")
    dest_dir = DIST / "rust"
    dest_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dest_dir / src.name)


def ensure_tool(tool: str) -> None:
    if shutil.which(tool) is None:
        raise SystemExit(f"required tool '{tool}' not found on PATH")


def package_python(version: str) -> None:
    ensure_tool("maturin")
    python_dist = DIST / "python"
    python_dist.mkdir(parents=True, exist_ok=True)
    run([
        "maturin",
        "build",
        "--release",
        "-m",
        "bindings/python/Cargo.toml",
        "-o",
        str(python_dist),
    ])
    wheels = list(python_dist.glob("tiletangle-*.whl"))
    if not wheels:
        raise FileNotFoundError("maturin build did not produce a wheel")
    wheel = wheels[0]
    venv_dir = DIST / "python-test-env"
    if venv_dir.exists():
        shutil.rmtree(venv_dir)
    run([sys.executable, "-m", "venv", str(venv_dir)])
    pip = venv_dir / "bin" / "pip"
    python = venv_dir / "bin" / "python"
    if platform.system() == "Windows":
        pip = venv_dir / "Scripts" / "pip.exe"
        python = venv_dir / "Scripts" / "python.exe"
    run([str(pip), "install", "--upgrade", "pip"], cwd=ROOT)
    run([str(pip), "install", str(wheel)], cwd=ROOT)
    smoke = """
import json
from tiletangle import Game
cfg = {
    "tileset": {"tile_kinds": [
        {"id": "A", "symbol": "A", "score": 1, "count": 10},
        {"id": "B", "symbol": "B", "score": 3, "count": 10}
    ]},
    "rack_size": 7,
    "board_layout": {"width": 5, "height": 5},
    "ruleset_id": "cross",
    "dictionary_id": "en",
    "rng_seed": 7,
    "tile_counts": {"A": 10, "B": 10},
    "free_word_mode": True
}
g = Game(json.dumps(cfg), 2)
assert g is not None
print("python smoke ok")
"""
    run([str(python), "-c", smoke], cwd=ROOT)
    shutil.rmtree(venv_dir, ignore_errors=True)


def package_wasm(version: str) -> None:
    ensure_tool("wasm-pack")
    ensure_tool("npm")
    pkg_dir = ROOT / "wasm" / "pkg"
    if pkg_dir.exists():
        shutil.rmtree(pkg_dir)
    for old in pkg_dir.glob("*.tgz"):
        old.unlink()
    run([
        "wasm-pack",
        "build",
        "--release",
        "--target",
        "bundler",
        "--out-dir",
        "pkg",
    ], cwd=ROOT / "wasm")
    pkg_package = pkg_dir / "package.json"
    pkg_readme = pkg_dir / "README.md"
    shutil.copy2(ROOT / "wasm" / "README.md", pkg_readme)
    pkg_data = json.loads(pkg_package.read_text())
    pkg_data.update(
        {
            "name": "@tiletangle/engine-wasm",
            "description": "WebAssembly build of the TileTangle universal word-game engine",
            "license": "MIT OR Apache-2.0",
            "repository": {"type": "git", "url": "https://example.com/TileTangle.git"},
            "bugs": {"url": "https://example.com/TileTangle/issues"},
            "exports": {
                ".": {
                    "import": "./tiletangle_wasm.js",
                    "types": "./tiletangle_wasm.d.ts",
                }
            },
        }
    )
    files = set(pkg_data.get("files", []))
    files.add("README.md")
    pkg_data["files"] = sorted(files)
    pkg_package.write_text(json.dumps(pkg_data, indent=2) + "\n")
    npm_dist = DIST / "npm"
    npm_dist.mkdir(parents=True, exist_ok=True)
    run(["npm", "pack"], cwd=pkg_dir)
    archives = sorted(pkg_dir.glob("*.tgz"))
    if not archives:
        raise FileNotFoundError("npm pack did not produce a tarball")
    archive = archives[-1]
    shutil.move(str(archive), npm_dist / archive.name)
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        run(["npm", "init", "-y"], cwd=tmp_path)
        pkg_json = tmp_path / "package.json"
        data = pkg_json.read_text()
        pkg_json.write_text(data.replace("\"main\": \"index.js\"", "\"type\": \"module\""))
        run(["npm", "install", str(npm_dist / archive.name)], cwd=tmp_path)
        node_cmd = [
            "node",
            "--experimental-wasm-modules",
            "--input-type=module",
            "-e",
            "import('@tiletangle/engine-wasm').then(m => { if (!m.generate_moves) throw new Error('missing API'); console.log('wasm smoke ok'); });",
        ]
        run(node_cmd, cwd=tmp_path)


def detect_platform() -> tuple[str, str]:
    system = platform.system()
    if system == "Linux":
        return "linux-x86_64", "libtiletangle_ffi.so"
    if system == "Darwin":
        return "macos-universal", "libtiletangle_ffi.dylib"
    if system == "Windows":
        return "windows-x86_64", "tiletangle_ffi.dll"
    raise SystemExit(f"unsupported platform: {system}")


def detect_godot_platform() -> tuple[str, str]:
    system = platform.system()
    if system == "Linux":
        return "linux-x86_64", "libtiletangle_godot.so"
    if system == "Darwin":
        return "macos-universal", "libtiletangle_godot.dylib"
    if system == "Windows":
        return "windows-x86_64", "tiletangle_godot.dll"
    raise SystemExit(f"unsupported platform: {system}")


def copytree(src: Path, dst: Path) -> None:
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(src, dst)


def package_unity(version: str) -> None:
    run(["cargo", "build", "-p", "tiletangle-engine-ffi", "--release"])
    platform_dir, lib_name = detect_platform()
    with tempfile.TemporaryDirectory() as temp_dir:
        staging = Path(temp_dir) / "com.tiletangle.engine"
        staging.mkdir(parents=True, exist_ok=True)
        copytree(ROOT / "bindings/unity/Runtime", staging / "Runtime")
        shutil.copy2(ROOT / "bindings/unity/README.md", staging / "README.md")
        shutil.copy2(ROOT / "bindings/unity/package.json", staging / "package.json")
        samples_root = staging / "Samples~" / "BoardDemo"
        samples_root.mkdir(parents=True, exist_ok=True)
        for item in (ROOT / "bindings/unity/Examples").iterdir():
            dest = samples_root / item.name
            if item.is_dir():
                copytree(item, dest)
            else:
                shutil.copy2(item, dest)
        plugins_dir = staging / "Plugins" / platform_dir
        plugins_dir.mkdir(parents=True, exist_ok=True)
        built_lib = ROOT / "target" / "release" / lib_name
        if not built_lib.exists():
            raise FileNotFoundError(f"expected Unity library at {built_lib}")
        shutil.copy2(built_lib, plugins_dir / lib_name)
        unity_dist = DIST / "unity"
        unity_dist.mkdir(parents=True, exist_ok=True)
        zip_base = unity_dist / f"tiletangle-unity-{version}"
        shutil.make_archive(str(zip_base), "zip", root_dir=staging)
        shutil.make_archive(str(zip_base), "gztar", root_dir=staging)


def package_godot(version: str) -> None:
    run(["cargo", "build", "-p", "tiletangle-godot", "--release"])
    platform_dir, lib_name = detect_godot_platform()
    with tempfile.TemporaryDirectory() as temp_dir:
        staging = Path(temp_dir) / "tiletangle-godot"
        staging.mkdir(parents=True, exist_ok=True)
        addons_src = ROOT / "bindings/godot/addons"
        copytree(addons_src, staging / "addons")
        gitkeep = staging / "addons" / "tiletangle" / "bin" / ".gitkeep"
        if gitkeep.exists():
            gitkeep.unlink()
        bin_dir = staging / "addons" / "tiletangle" / "bin" / platform_dir
        bin_dir.mkdir(parents=True, exist_ok=True)
        built_lib = ROOT / "target" / "release" / lib_name
        if not built_lib.exists():
            raise FileNotFoundError(f"expected Godot library at {built_lib}")
        shutil.copy2(built_lib, bin_dir / lib_name)
        samples_dir = staging / "samples"
        copytree(ROOT / "bindings/godot/examples", samples_dir)
        godot_dist = DIST / "godot"
        godot_dist.mkdir(parents=True, exist_ok=True)
        zip_base = godot_dist / f"tiletangle-godot-{version}"
        shutil.make_archive(str(zip_base), "zip", root_dir=staging)


def write_manifest(version: str) -> None:
    manifest = {
        "version": version,
        "artifacts": {
            "rust": sorted([p.name for p in (DIST / "rust").glob("*")]) if (DIST / "rust").exists() else [],
            "python": sorted([p.name for p in (DIST / "python").glob("*")]) if (DIST / "python").exists() else [],
            "npm": sorted([p.name for p in (DIST / "npm").glob("*")]) if (DIST / "npm").exists() else [],
            "unity": sorted([p.name for p in (DIST / "unity").glob("*")]) if (DIST / "unity").exists() else [],
            "godot": sorted([p.name for p in (DIST / "godot").glob("*")]) if (DIST / "godot").exists() else [],
        },
    }
    (DIST / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip", choices=["rust", "python", "npm", "unity", "godot"], action="append", help="Skip selected packaging steps")
    args = parser.parse_args()
    version = read_version()
    clean_dist()
    if not args.skip or "rust" not in args.skip:
        package_rust(version)
    if not args.skip or "python" not in args.skip:
        package_python(version)
    if not args.skip or "npm" not in args.skip:
        package_wasm(version)
    if not args.skip or "unity" not in args.skip:
        package_unity(version)
    if not args.skip or "godot" not in args.skip:
        package_godot(version)
    write_manifest(version)
    print("Artifacts ready under", DIST)


if __name__ == "__main__":
    main()
