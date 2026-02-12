param(
    [string]$Scope = "visible_above_fold",
    [string]$DateOverride = ""
)

$browsy = 'C:\Users\Cade\projects\agentbrowser\target\release\browsy.exe'
$sites = @(
    'https://news.ycombinator.com',
    'https://github.com/login',
    'https://accounts.google.com',
    'https://duckduckgo.com',
    'https://en.wikipedia.org/wiki/Main_Page',
    'https://www.bbc.com/news',
    'https://stackoverflow.com/questions',
    'https://craigslist.org',
    'https://github.com/anthropics/claude-code',
    'https://www.amazon.com',
    'https://news.ycombinator.com/login',
    'https://httpbin.org/forms/post',
    'https://example.com',
    'https://www.python.org'
)

$maxMs = 1000
$minLines = 10
$minChars = 400

$flags = @()
if ($Scope -eq "visible") { $flags += "--visible-only" }
if ($Scope -eq "above_fold") { $flags += "--above-fold" }
if ($Scope -eq "visible_above_fold") { $flags += "--visible-only"; $flags += "--above-fold" }

$rows = @()
foreach ($url in $sites) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $output = & $browsy fetch $url @flags 2>&1 | Out-String
    $ms = $sw.ElapsedMilliseconds
    $chars = $output.Length
    $lines = ($output -split "`n").Count

    $status = "OK"
    $note = ""
    if ($output -match "Error:") {
        $status = "NEEDS_HUMAN"
        $note = "fetch_error"
    } elseif ($lines -lt $minLines -or $chars -lt $minChars) {
        $status = "NEEDS_HUMAN"
        $note = "likely_blocked_or_empty"
    } elseif ($ms -gt $maxMs) {
        $status = "WARN"
        $note = "slow_fetch_or_parse"
    }

    $rows += [pscustomobject]@{
        site = $url
        scope = $Scope
        ms = $ms
        lines = $lines
        chars = $chars
        status = $status
        note = $note
    }
}

$date = if ($DateOverride) { $DateOverride } else { (Get-Date -Format "yyyy-MM-dd") }
$outDir = "C:\Users\Cade\Projects\agentbrowser\docs\reports"
$csvPath = Join-Path $outDir "$date-browsy-live-$Scope.csv"
$jsonPath = Join-Path $outDir "$date-browsy-live-$Scope.json"

$rows | Export-Csv -NoTypeInformation -Path $csvPath
$rows | ConvertTo-Json -Depth 4 | Out-File -FilePath $jsonPath -Encoding utf8

Write-Output "Wrote $csvPath"
Write-Output "Wrote $jsonPath"
