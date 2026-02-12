param(
    [int]$Instances = 10,
    [int]$Iterations = 1,
    [string]$InputHtml = "C:\\Users\\Cade\\Projects\\agentbrowser\\crates\\core\\tests\\corpus\\snapshots\\wikipedia-rust.html",
    [string]$Url = "https://httpbin.org/forms/post",
    [string]$DateOverride = "",
    [string]$Mode = "parse"
)

$browsy = 'C:\Users\Cade\projects\agentbrowser\target\release\browsy.exe'
$date = if ($DateOverride) { $DateOverride } else { (Get-Date -Format "yyyy-MM-dd") }
$outDir = "C:\Users\Cade\Projects\agentbrowser\docs\reports"
$null = New-Item -ItemType Directory -Force -Path $outDir
$csvPath = Join-Path $outDir "$date-browsy-parallel-$Mode-i$Instances.csv"
$jsonPath = Join-Path $outDir "$date-browsy-parallel-$Mode-i$Instances.json"

function Start-One([string]$cmd, [string]$argString) {
    $tmpOut = [System.IO.Path]::GetTempFileName()
    $tmpErr = [System.IO.Path]::GetTempFileName()
    Start-Process -FilePath $cmd -ArgumentList $argString -NoNewWindow -PassThru -RedirectStandardOutput $tmpOut -RedirectStandardError $tmpErr
}

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

$rows = @()
for ($iter = 1; $iter -le $Iterations; $iter++) {
    $procs = @()
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    for ($i = 0; $i -lt $Instances; $i++) {
        if ($Mode -eq "parse") {
            $argString = "parse `"$InputHtml`" --viewport 1920x1080"
            $procs += Start-One $browsy $argString
        } else {
            $argString = "fetch `"$Url`" --viewport 1920x1080 --no-css"
            $procs += Start-One $browsy $argString
        }
    }

    $peakMem = @{}
    $peakTreeMem = @{}
    foreach ($p in $procs) {
        try {
            $proc = Get-Process -Id $p.Id -ErrorAction Stop
            $peakMem[$p.Id] = $proc.PeakWorkingSet64
            $treePeak = Get-ProcessTreePeak $p.Id
            if ($proc.PeakWorkingSet64 -gt $treePeak) {
                $treePeak = $proc.PeakWorkingSet64
            }
            $peakTreeMem[$p.Id] = $treePeak
        } catch {}
    }
    $active = $true
    while ($active) {
        $active = $false
        foreach ($p in $procs) {
            if (!$p.HasExited) {
                $active = $true
                try {
                    $proc = Get-Process -Id $p.Id -ErrorAction Stop
                    if (!$peakMem.ContainsKey($p.Id) -or $proc.PeakWorkingSet64 -gt $peakMem[$p.Id]) {
                        $peakMem[$p.Id] = $proc.PeakWorkingSet64
                    }
                    $treePeak = Get-ProcessTreePeak $p.Id
                    if ($proc.PeakWorkingSet64 -gt $treePeak) {
                        $treePeak = $proc.PeakWorkingSet64
                    }
                    if (!$peakTreeMem.ContainsKey($p.Id) -or $treePeak -gt $peakTreeMem[$p.Id]) {
                        $peakTreeMem[$p.Id] = $treePeak
                    }
                } catch {}
            }
        }
        Start-Sleep -Milliseconds 50
    }
    $sw.Stop()

    $peakBytes = if ($peakMem.Count -gt 0) { ($peakMem.Values | Measure-Object -Sum).Sum } else { 0 }
    $avgPeakMb = if ($peakMem.Count -gt 0) { ($peakMem.Values | Measure-Object -Average).Average / 1MB } else { 0 }
    $treeSum = if ($peakTreeMem.Count -gt 0) { ($peakTreeMem.Values | Measure-Object -Sum).Sum } else { 0 }
    if ($treeSum -eq 0 -and $peakMem.Count -gt 0) {
        $peakTreeMem = $peakMem
        $treeSum = ($peakTreeMem.Values | Measure-Object -Sum).Sum
    }
    $peakTreeBytes = $treeSum
    $avgPeakTreeMb = if ($peakTreeMem.Count -gt 0) { ($peakTreeMem.Values | Measure-Object -Average).Average / 1MB } else { 0 }

    $rows += [pscustomobject]@{
        date = $date
        mode = $Mode
        instances = $Instances
        iteration = $iter
        wall_ms = $sw.ElapsedMilliseconds
        peak_working_set_mb_total = [math]::Round(($peakBytes / 1MB), 2)
        peak_working_set_mb_avg = [math]::Round($avgPeakMb, 2)
        peak_working_set_mb_total_tree = [math]::Round(($peakTreeBytes / 1MB), 2)
        peak_working_set_mb_avg_tree = [math]::Round($avgPeakTreeMb, 2)
        input_html = if ($Mode -eq "parse") { $InputHtml } else { "" }
        url = if ($Mode -eq "fetch") { $Url } else { "" }
    }
}

$rows | Export-Csv -NoTypeInformation -Path $csvPath
$rows | ConvertTo-Json -Depth 4 | Out-File -FilePath $jsonPath -Encoding utf8

Write-Output "Wrote $csvPath"
Write-Output "Wrote $jsonPath"
