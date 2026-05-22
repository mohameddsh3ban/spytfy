import { execSync } from 'child_process';

const PORTS = [4200, 1420];
const PROCESSES = [
  'spytfy-app.exe',
  'yt-dlp-x86_64-pc-windows-gnu.exe',
  'yt-dlp-x86_64-pc-windows-msvc.exe',
  'ffmpeg-x86_64-pc-windows-gnu.exe',
  'ffmpeg-x86_64-pc-windows-msvc.exe',
];

function kill(cmd, label) {
  try {
    execSync(cmd, { stdio: 'pipe' });
    console.log(`  Killed: ${label}`);
  } catch {}
}

console.log('\n=== Stopping Spytfy ===\n');

// Kill processes on dev ports
console.log('[1/4] Killing dev server ports...');
for (const port of PORTS) {
  try {
    const out = execSync(
      `netstat -ano | findstr :${port} | findstr LISTENING`,
      { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }
    );
    const pids = new Set(
      out.split('\n').map(l => l.trim().split(/\s+/).pop()).filter(p => p && p !== '0')
    );
    for (const pid of pids) kill(`taskkill /F /PID ${pid}`, `PID ${pid} (port ${port})`);
  } catch {}
}

// Kill app and sidecar processes
console.log('[2/4] Killing app & sidecar processes...');
for (const name of PROCESSES) {
  kill(`taskkill /F /IM ${name}`, name);
}

// Stop NX daemon
console.log('[3/4] Stopping NX daemon...');
try {
  execSync('npx nx daemon --stop', { stdio: 'pipe', cwd: process.cwd() + '/spytfy' });
  console.log('  Killed: nx daemon');
} catch {}

// Clean temp files
console.log('[4/4] Cleaning temp files...');
try {
  execSync('del /s /q *.webm.part 2>nul', { cwd: process.cwd() + '/spytfy', stdio: 'pipe', shell: true });
} catch {}

console.log('\n=== All stopped ===\n');
