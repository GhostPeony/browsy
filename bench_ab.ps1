$env:NPM_CONFIG_YES = "true"
npx --yes agent-browser --version 2>&1 | Out-Null
npx --yes agent-browser close 2>&1 | Out-Null
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
Write-Output "SITE|MS|LINES|CHARS"
foreach ($url in $sites) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    npx --yes agent-browser open $url 2>&1 | Out-Null
    $snap = npx --yes agent-browser snapshot -i 2>&1 | Out-String
    $ms = $sw.ElapsedMilliseconds
    $chars = $snap.Length
    $lines = ($snap -split "`n").Count
    Write-Output "${url}|${ms}|${lines}|${chars}"
}
