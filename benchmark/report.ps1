param(
    [string]$Manifest = "C:\\Users\\Cade\\Projects\\agentbrowser\\benchmark\\manifest.json",
    [string]$DateOverride = ""
)

$doc = [System.Text.Json.JsonDocument]::Parse((Get-Content -LiteralPath $Manifest -Raw))
$root = $doc.RootElement
$iterations = $root.GetProperty("iterations").GetInt32()
$instances = @()
foreach ($v in $root.GetProperty("parallel_instances").EnumerateArray()) {
    $instances += $v.GetInt32()
}
$date = if ($DateOverride) { $DateOverride } else { (Get-Date -Format "yyyy-MM-dd") }
$outDir = "C:\\Users\\Cade\\Projects\\agentbrowser\\docs\\reports"
$null = New-Item -ItemType Directory -Force -Path $outDir

function Read-Avg([string]$path, [string]$field) {
    if (!(Test-Path $path)) { return $null }
    $rows = Import-Csv $path
    if ($rows.Count -eq 0) { return $null }
    $avg = ($rows | Measure-Object -Property $field -Average).Average
    [math]::Round($avg, 2)
}

function Read-P95([string]$path, [string]$field) {
    if (!(Test-Path $path)) { return $null }
    $rows = Import-Csv $path | Sort-Object { [double]$_.$field }
    if ($rows.Count -eq 0) { return $null }
    $idx = [math]::Ceiling($rows.Count * 0.95) - 1
    $val = [double]$rows[$idx].$field
    [math]::Round($val, 2)
}

$summary = @()
foreach ($i in $instances) {
    $summary += [pscustomobject]@{
        bench = "browsy-parse"
        instances = $i
        avg_wall_ms = Read-Avg "$outDir\\$date-browsy-parallel-parse-i$i.csv" "wall_ms"
    }
    $summary += [pscustomobject]@{
        bench = "browsy-fetch"
        instances = $i
        avg_wall_ms = Read-Avg "$outDir\\$date-browsy-parallel-fetch-i$i.csv" "wall_ms"
    }
    $summary += [pscustomobject]@{
        bench = "agent-browser"
        instances = $i
        avg_wall_ms = Read-Avg "$outDir\\$date-agent-browser-parallel-i$i.csv" "wall_ms"
    }
    $summary += [pscustomobject]@{
        bench = "playwright"
        instances = $i
        avg_wall_ms = Read-Avg "$outDir\\$date-playwright-parallel-i$i.csv" "wall_ms"
    }
}

$summaryPath = "$outDir\\$date-benchmark-summary.csv"
$summary | Export-Csv -NoTypeInformation -Path $summaryPath

$mdPath = "$outDir\\$date-benchmark-summary.md"
@"
# Benchmark Summary ($date)

Generated from: $Manifest
Iterations: $iterations
Instances: $($instances -join ',')

| Benchmark | Instances | Avg wall (ms) |
| --- | ---: | ---: |
$(($summary | ForEach-Object { "| $($_.bench) | $($_.instances) | $($_.avg_wall_ms) |" }) -join "`n")

"@ | Out-File -FilePath $mdPath -Encoding utf8

$latestPath = "$outDir\\latest.md"
$bullets = @()
$b1 = $summary | Where-Object { $_.bench -eq "browsy-parse" -and $_.instances -eq 1 }
$b5 = $summary | Where-Object { $_.bench -eq "browsy-parse" -and $_.instances -eq 5 }
$b10 = $summary | Where-Object { $_.bench -eq "browsy-parse" -and $_.instances -eq 10 }
$f1 = $summary | Where-Object { $_.bench -eq "browsy-fetch" -and $_.instances -eq 1 }
$f5 = $summary | Where-Object { $_.bench -eq "browsy-fetch" -and $_.instances -eq 5 }
$f10 = $summary | Where-Object { $_.bench -eq "browsy-fetch" -and $_.instances -eq 10 }
$ab1 = $summary | Where-Object { $_.bench -eq "agent-browser" -and $_.instances -eq 1 }
$ab5 = $summary | Where-Object { $_.bench -eq "agent-browser" -and $_.instances -eq 5 }
$ab10 = $summary | Where-Object { $_.bench -eq "agent-browser" -and $_.instances -eq 10 }
$pw1 = $summary | Where-Object { $_.bench -eq "playwright" -and $_.instances -eq 1 }
$pw5 = $summary | Where-Object { $_.bench -eq "playwright" -and $_.instances -eq 5 }
$pw10 = $summary | Where-Object { $_.bench -eq "playwright" -and $_.instances -eq 10 }

if ($b1) { $bullets += "- Parse i1: $($b1.avg_wall_ms) ms (browsy)" }
if ($b5) { $bullets += "- Parse i5: $($b5.avg_wall_ms) ms (browsy)" }
if ($b10) { $bullets += "- Parse i10: $($b10.avg_wall_ms) ms (browsy)" }
if ($f1) { $bullets += "- Fetch i1: $($f1.avg_wall_ms) ms (browsy)" }
if ($f5) { $bullets += "- Fetch i5: $($f5.avg_wall_ms) ms (browsy)" }
if ($f10) { $bullets += "- Fetch i10: $($f10.avg_wall_ms) ms (browsy)" }
if ($ab1) { $bullets += "- Agent-browser i1: $($ab1.avg_wall_ms) ms" }
if ($ab5) { $bullets += "- Agent-browser i5: $($ab5.avg_wall_ms) ms" }
if ($ab10) { $bullets += "- Agent-browser i10: $($ab10.avg_wall_ms) ms" }
if ($pw1) { $bullets += "- Playwright i1: $($pw1.avg_wall_ms) ms" }
if ($pw5) { $bullets += "- Playwright i5: $($pw5.avg_wall_ms) ms" }
if ($pw10) { $bullets += "- Playwright i10: $($pw10.avg_wall_ms) ms" }

@"
# Latest Benchmark Snapshot ($date)

Source: $mdPath
Iterations: $iterations
Instances: $($instances -join ',')

$($bullets -join "`n")

"@ | Out-File -FilePath $latestPath -Encoding utf8

Write-Output "Wrote $summaryPath"
Write-Output "Wrote $mdPath"
Write-Output "Wrote $latestPath"
