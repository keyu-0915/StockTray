param(
  [switch] $RequireFullDay
)

$ErrorActionPreference = "Stop"
$statePath = Join-Path $env:APPDATA "StockTray\market-snapshots.json"
if (-not (Test-Path -LiteralPath $statePath)) {
  throw "Market snapshot file was not found: $statePath"
}

$state = Get-Content -Raw -Encoding UTF8 -LiteralPath $statePath | ConvertFrom-Json
$history = @($state.history | Where-Object { $_.phase -in @("auction_final", "opening_observation", "continuous") })
if ($state.algorithm_version -ne "2.2.0") {
  throw "Expected algorithm 2.2.0, found $($state.algorithm_version)"
}
if (-not $state.current.quality.conclusion_ready) {
  throw "Current snapshot is not conclusion-ready: $($state.current.quality.index_error)"
}
if ($history.Count -eq 0) {
  throw "No quality-approved market evidence has been persisted"
}

$times = @($history | ForEach-Object { [TimeSpan]::Parse($_.time) })
for ($index = 1; $index -lt $times.Count; $index++) {
  if ($times[$index] -lt $times[$index - 1]) {
    throw "History is not chronological at $($history[$index].time)"
  }
}
foreach ($item in $history) {
  if (@($item.scores).Count -ne 3 -or @($item.preferences).Count -ne 3 -or
      @($item.cap_weight_returns).Count -ne 3 -or @($item.equal_weight_returns).Count -ne 3) {
    throw "Incomplete diagnostic arrays at $($item.time)"
  }
  if ($item.coverage -lt 80 -or $item.minimum_style_coverage -lt 80) {
    throw "Low-quality point was persisted at $($item.time)"
  }
}

if ($RequireFullDay) {
  $continuous = @($history | Where-Object { $_.phase -in @("opening_observation", "continuous") })
  if ($continuous.Count -lt 40) {
    throw "A 5-minute full-day run needs at least 40 valid continuous points; found $($continuous.Count)"
  }
  if ([TimeSpan]::Parse($continuous[0].time) -gt [TimeSpan]::Parse("09:35:00")) {
    throw "The first continuous point is too late: $($continuous[0].time)"
  }
  if ([TimeSpan]::Parse($continuous[-1].time) -lt [TimeSpan]::Parse("14:55:00")) {
    throw "The last continuous point is too early: $($continuous[-1].time)"
  }
  if (-not ($continuous | Where-Object { [TimeSpan]::Parse($_.time) -ge [TimeSpan]::Parse("13:00:00") })) {
    throw "No afternoon evidence was recorded"
  }
}

$history | Select-Object time, phase, status, coverage, minimum_style_coverage,
  @{Name="scores"; Expression={ $_.scores -join "/" }},
  @{Name="cap_minus_equal"; Expression={
    $point = $_
    (0..2 | ForEach-Object { [math]::Round($point.cap_weight_returns[$_] - $point.equal_weight_returns[$_], 3) }) -join "/"
  }} | Format-Table -AutoSize

Write-Host "Market session validation passed with $($history.Count) quality-approved points."
