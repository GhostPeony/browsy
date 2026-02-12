param(
    [string]$DateOverride = ""
)

$date = if ($DateOverride) { $DateOverride } else { (Get-Date -Format "yyyy-MM-dd") }
$outDir = "C:\Users\Cade\Projects\agentbrowser\docs\reports"
$null = New-Item -ItemType Directory -Force -Path $outDir
$csvPath = Join-Path $outDir "$date-govform-lab.csv"

$browsy = 'C:\Users\Cade\projects\agentbrowser\target\release\browsy.exe'
$base = "http://127.0.0.1:8008"
$paths = @(
    "/forms/sign-in.html",
    "/forms/name.html",
    "/forms/benefits-step1.html",
    "/forms/benefits-step2.html",
    "/forms/health-intake.html",
    "/forms/consent.html",
    "/forms/upload.html",
    "/forms/errors.html"
)

$rows = @()
try {
    Invoke-WebRequest -Uri "$base/" -UseBasicParsing | Out-Null
    $useFetch = $true
} catch {
    $useFetch = $false
}

foreach ($p in $paths) {
    $url = "$base$p"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    if ($useFetch) {
        & $browsy fetch $url --no-css --allow-private-network | Out-Null
    } else {
        $local = "C:\Users\Cade\Projects\agentbrowser\benchmark\govform-lab" + $p.Replace("/", "\")
        & $browsy parse $local | Out-Null
    }
    $sw.Stop()
    $rows += [pscustomobject]@{
        date = $date
        url = $url
        wall_ms = $sw.ElapsedMilliseconds
        mode = if ($useFetch) { "fetch" } else { "parse" }
    }
}

$rows | Export-Csv -NoTypeInformation -Path $csvPath
Write-Output "Wrote $csvPath"
