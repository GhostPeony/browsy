npx agent-browser close 2>&1 | Out-Null
$sw = [System.Diagnostics.Stopwatch]::StartNew()
npx agent-browser open "https://news.ycombinator.com" 2>&1 | Out-Null
$snap = npx agent-browser snapshot -i 2>&1 | Out-String
$ms = $sw.ElapsedMilliseconds
Write-Output "HN|${ms}|$(($snap -split "`n").Count)|$($snap.Length)"

$sw = [System.Diagnostics.Stopwatch]::StartNew()
npx agent-browser open "https://github.com/login" 2>&1 | Out-Null
$snap = npx agent-browser snapshot -i 2>&1 | Out-String
$ms = $sw.ElapsedMilliseconds
Write-Output "GH-Login|${ms}|$(($snap -split "`n").Count)|$($snap.Length)"

$sw = [System.Diagnostics.Stopwatch]::StartNew()
npx agent-browser open "https://example.com" 2>&1 | Out-Null
$snap = npx agent-browser snapshot -i 2>&1 | Out-String
$ms = $sw.ElapsedMilliseconds
Write-Output "Example|${ms}|$(($snap -split "`n").Count)|$($snap.Length)"

npx agent-browser close 2>&1 | Out-Null
