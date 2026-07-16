import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const iconDir = path.join(root, 'src-tauri', 'icons');
const source = path.join(iconDir, 'icon.svg');
const output = fs.mkdtempSync(path.join(os.tmpdir(), 'stocktray-icons-'));
const tauriCli = path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');

try {
  execFileSync(process.execPath, [tauriCli, 'icon', source, '--output', output], {
    cwd: root,
    stdio: 'inherit',
  });

  fs.copyFileSync(path.join(output, 'icon.ico'), path.join(iconDir, 'icon.ico'));
  fs.copyFileSync(path.join(output, '32x32.png'), path.join(iconDir, '32x32.png'));
  fs.copyFileSync(path.join(output, 'icon.png'), path.join(iconDir, 'icon-preview.png'));
  console.log(`Wrote ${path.join(iconDir, 'icon.ico')}`);
  console.log(`Wrote ${path.join(iconDir, '32x32.png')}`);
  console.log(`Wrote ${path.join(iconDir, 'icon-preview.png')}`);
} finally {
  fs.rmSync(output, { recursive: true, force: true });
}
