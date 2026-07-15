$ErrorActionPreference = 'Stop'

$keyPath = Join-Path $HOME '.ssh\finagents_550w'
$sshArgs = @(
    '-t'
    '-i'
    $keyPath
    'root@550W'
    'cd /opt/stocktray-opend && ./configure-secrets.sh'
)

& ssh.exe @sshArgs
if ($LASTEXITCODE -ne 0) {
    Write-Host "`nConfiguration failed. SSH exit code: $LASTEXITCODE" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "`nOpenD credentials configured. Return to Codex and confirm completion." -ForegroundColor Green
