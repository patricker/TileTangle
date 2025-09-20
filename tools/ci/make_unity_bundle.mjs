#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import {execSync} from 'node:child_process';

const root = process.cwd();
const artifactsDir = path.join(root, 'dist', 'artifacts');
const outRoot = path.join(root, 'dist', 'unity-bundle');
const pkgDir = path.join(root, 'bindings', 'unity');

function ensureDir(p) { fs.mkdirSync(p, {recursive: true}); }
function copyFile(src, dst) { ensureDir(path.dirname(dst)); fs.copyFileSync(src, dst); }
function copyDir(src, dst) {
  ensureDir(dst);
  for (const entry of fs.readdirSync(src, {withFileTypes: true})) {
    if (entry.name === 'node_modules' || entry.name === 'build' || entry.name === 'dist') continue;
    // Exclude Unity PlayMode tests from shipped package by default
    if (entry.isDirectory() && entry.name === 'Tests') continue;
    const s = path.join(src, entry.name);
    const d = path.join(dst, entry.name);
    if (entry.isDirectory()) copyDir(s, d); else copyFile(s, d);
  }
}

function main() {
  const unityPkgJson = JSON.parse(fs.readFileSync(path.join(pkgDir, 'package.json'), 'utf8'));
  const version = unityPkgJson.version || '0.0.0';

  // Prepare output structure
  ensureDir(outRoot);
  const outPkg = path.join(outRoot, 'com.tiletangle.engine');
  copyDir(pkgDir, outPkg);

  // Copy native libs from downloaded artifacts
  const linuxLib = path.join(artifactsDir, 'ffi-linux-stable', 'libtiletangle_ffi.so');
  const macLib = path.join(artifactsDir, 'ffi-macos-stable', 'libtiletangle_ffi.dylib');
  const winLib = path.join(artifactsDir, 'ffi-windows-msvc-stable', 'tiletangle_ffi.dll');
  const header = path.join(artifactsDir, 'ffi-header', 'engine.h');

  const pluginsRoot = path.join(outRoot, 'Plugins');
  if (fs.existsSync(linuxLib)) copyFile(linuxLib, path.join(pluginsRoot, 'Linux', 'libtiletangle_ffi.so'));
  if (fs.existsSync(macLib)) copyFile(macLib, path.join(pluginsRoot, 'macOS', 'libtiletangle_ffi.dylib'));
  if (fs.existsSync(winLib)) copyFile(winLib, path.join(pluginsRoot, 'Windows', 'tiletangle_ffi.dll'));
  if (fs.existsSync(header)) copyFile(header, path.join(pluginsRoot, 'include', 'engine.h'));

  // Add bundle README with quick install steps
  const readme = `Unity Bundle\n\nContents:\n- com.tiletangle.engine/ — Unity UPM package with C# wrapper and sample scripts.\n- Plugins/ — prebuilt native libraries for Windows, macOS, and Linux, plus C header.\n\nQuick start:\n1) In Unity, copy the appropriate native library from Plugins/<Platform>/ into Assets/Plugins/<Platform>/.\n2) In Package Manager, add package from disk… and select the com.tiletangle.engine folder.\n3) Create an empty scene and add the sample script (BoardDemo) to an empty GameObject, then press Play.\n\nNotes:\n- For production, consider per-platform plugin import settings and meta files.\n- The UPM package includes Runtime/TileTangle.cs (P/Invoke wrapper) and example UI scripts.\n`;
  fs.writeFileSync(path.join(outRoot, 'README.md'), readme);

  // Create archive
  const archive = path.join(root, 'dist', `unity-bundle-v${version}.tar.gz`);
  const cwd = path.join(root, 'dist');
  execSync(`tar -czf ${path.basename(archive)} unity-bundle`, {cwd, stdio: 'inherit'});
  console.log(`Created ${archive}`);
}

main();

