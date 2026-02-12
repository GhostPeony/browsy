npx agent-browser close 2>&1 | Out-Null
$sites = @(
    'https://news.ycombinator.com',
    'https://github.com/login',
    'https://duckduckgo.com',
    'https://example.com',
    'https://httpbin.org/forms/post'
)
Write-Output "SITE|MS|LINES|CHARS"
foreach ($url in $sites) {
    try {
        $job = Start-Job -ScriptBlock {
            param($u)
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            npx agent-browser open $u 2>&1 | Out-Null
            $snap = npx agent-browser snapshot -i 2>&1 | Out-String
            $ms = $sw.ElapsedMilliseconds
            $chars = $snap.Length
            $lines = ($snap -split "`n").Count
            "$u|$ms|$lines|$chars"
        } -ArgumentList $url
        $result = $job | Wait-Job -Timeout 30 | Receive-Job
        if ($result) {
            Write-Output $result
        } else {
            Stop-Job $job -ErrorAction SilentlyContinue
            Remove-Job $job -Force -ErrorAction SilentlyContinue
            npx agent-browser close 2>&1 | Out-Null
            Write-Output "${url}|TIMEOUT|0|0"
        }
    } catch {
        Write-Output "${url}|ERROR|0|0"
    }
}
npx agent-browser close 2>&1 | Out-Null
