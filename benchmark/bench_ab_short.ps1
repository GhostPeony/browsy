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
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    npx agent-browser open $url 2>&1 | Out-Null
    $snap = npx agent-browser snapshot -i 2>&1 | Out-String
    $ms = $sw.ElapsedMilliseconds
    $chars = $snap.Length
    $lines = ($snap -split "`n").Count
    Write-Output "${url}|${ms}|${lines}|${chars}"
}
npx agent-browser close 2>&1 | Out-Null
