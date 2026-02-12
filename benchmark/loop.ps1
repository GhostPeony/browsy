param(
    [string]$Manifest = "C:\\Users\\Cade\\Projects\\agentbrowser\\benchmark\\manifest.json",
    [string]$DateOverride = ""
)

$manifestPath = $Manifest
if ($manifestPath -match '^\s*@\{') {
    throw "Manifest must be a file path, not a hashtable string."
}
$manifestRaw = Get-Content -LiteralPath $manifestPath -Raw
if ([string]::IsNullOrWhiteSpace($manifestRaw)) {
    throw "Manifest file is empty: $manifestPath"
}
$doc = [System.Text.Json.JsonDocument]::Parse($manifestRaw)
$root = $doc.RootElement
$date = if ($DateOverride) { $DateOverride } else { (Get-Date -Format "yyyy-MM-dd") }
$outDir = "C:\Users\Cade\Projects\agentbrowser\docs\reports"
$iters = $root.GetProperty("iterations").GetInt32()
$instances = @()
foreach ($v in $root.GetProperty("parallel_instances").EnumerateArray()) {
    $instances += $v.GetInt32()
}
$instances = $instances | Where-Object { $_ -gt 0 }
$parseInput = $root.GetProperty("browsy").GetProperty("parse_input_html").GetString()
$fetchUrl = $root.GetProperty("browsy").GetProperty("fetch_url").GetString()
$agentUrl = $root.GetProperty("competitors").GetProperty("agent_browser").GetProperty("url").GetString()
$playwrightUrl = $root.GetProperty("competitors").GetProperty("playwright").GetProperty("url").GetString()

Write-Output "=== browsy loop: $date ==="
Write-Output "iterations=$iters instances=$($instances -join ',')"
Write-Output "manifest=$manifestPath"

foreach ($i in $instances) {
    Write-Output "browsy parse i=$i"
    & "C:\Program Files\PowerShell\7\pwsh.exe" -Command ".\benchmark\bench_parallel.ps1 -Instances $i -Iterations $iters -Mode parse -InputHtml `"$parseInput`" -DateOverride $date" | Out-Null
    Write-Output "browsy fetch i=$i"
    & "C:\Program Files\PowerShell\7\pwsh.exe" -Command ".\benchmark\bench_parallel.ps1 -Instances $i -Iterations $iters -Mode fetch -Url `"$fetchUrl`" -DateOverride $date" | Out-Null
}

foreach ($i in $instances) {
    Write-Output "agent-browser i=$i"
    & "C:\Program Files\PowerShell\7\pwsh.exe" -Command ".\benchmark\bench_ab_parallel.ps1 -Instances $i -Iterations $iters -Url `"$agentUrl`" -DateOverride $date" | Out-Null
    Write-Output "playwright i=$i"
    & "C:\Program Files\PowerShell\7\pwsh.exe" -Command ".\benchmark\bench_playwright_parallel.ps1 -Instances $i -Iterations $iters -Url `"$playwrightUrl`" -DateOverride $date" | Out-Null
}

Write-Output "browsy live sites"
& "C:\Program Files\PowerShell\7\pwsh.exe" -Command ".\benchmark\bench_browsy_adaptive.ps1 -OutPath `"$outDir\\$date-browsy-live.txt`"" | Out-Null

& "C:\Program Files\PowerShell\7\pwsh.exe" -Command ".\benchmark\report.ps1 -Manifest `"$manifestPath`" -DateOverride $date"
