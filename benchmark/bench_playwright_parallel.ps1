param(
    [int]$Instances = 5,
    [int]$Iterations = 1,
    [string]$Url = "https://httpbin.org/forms/post",
    [string]$DateOverride = ""
)

$date = if ($DateOverride) { $DateOverride } else { (Get-Date -Format "yyyy-MM-dd") }
$outDir = "C:\Users\Cade\Projects\agentbrowser\docs\reports"
$null = New-Item -ItemType Directory -Force -Path $outDir
$csvPath = Join-Path $outDir "$date-playwright-parallel-i$Instances.csv"
$jsonPath = Join-Path $outDir "$date-playwright-parallel-i$Instances.json"

function Get-ProcessTreeIds([int]$rootId) {
    $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId
    $children = @{}
    foreach ($p in $all) {
        if (!$children.ContainsKey($p.ParentProcessId)) {
            $children[$p.ParentProcessId] = @()
        }
        $children[$p.ParentProcessId] += $p.ProcessId
    }
    $ids = @($rootId)
    for ($i = 0; $i -lt $ids.Count; $i++) {
        $pid = $ids[$i]
        if ($children.ContainsKey($pid)) {
            $ids += $children[$pid]
        }
    }
    $ids | Select-Object -Unique
}

function Get-ProcessTreePeak([int]$rootId) {
    $total = 0
    foreach ($pid in (Get-ProcessTreeIds $rootId)) {
        try {
            $proc = Get-Process -Id $pid -ErrorAction Stop
            $total += $proc.PeakWorkingSet64
        } catch {}
    }
    $total
}

$playwrightDir = "C:\Users\Cade\Projects\agentbrowser\benchmark\playwright"
$rows = @()
for ($iter = 1; $iter -le $Iterations; $iter++) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $tmpOut = [System.IO.Path]::GetTempFileName()
    $tmpErr = [System.IO.Path]::GetTempFileName()
    $p = Start-Process -FilePath "node" -ArgumentList "`"$playwrightDir\playwright_parallel.cjs`" --instances $Instances --iterations 1 --url `"$Url`" --date $date" -NoNewWindow -PassThru -RedirectStandardOutput $tmpOut -RedirectStandardError $tmpErr
    $peakTree = 0
    while (!$p.HasExited) {
        try {
            $treePeak = Get-ProcessTreePeak $p.Id
            try {
                $rootProc = Get-Process -Id $p.Id -ErrorAction Stop
                if ($rootProc.PeakWorkingSet64 -gt $treePeak) {
                    $treePeak = $rootProc.PeakWorkingSet64
                }
            } catch {}
            if ($treePeak -gt $peakTree) {
                $peakTree = $treePeak
            }
        } catch {}
        Start-Sleep -Milliseconds 50
    }
    $sw.Stop()

    $rows += [pscustomobject]@{
        date = $date
        mode = "playwright"
        instances = $Instances
        iteration = $iter
        wall_ms = $sw.ElapsedMilliseconds
        peak_working_set_mb_total_tree = [math]::Round(($peakTree / 1MB), 2)
        url = $Url
    }
}

$rows | Export-Csv -NoTypeInformation -Path $csvPath
$rows | ConvertTo-Json -Depth 4 | Out-File -FilePath $jsonPath -Encoding utf8

Write-Output "Wrote $csvPath"
Write-Output "Wrote $jsonPath"
