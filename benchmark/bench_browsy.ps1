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
Write-Output "SITE|MS|LINES|CHARS"
foreach ($url in $sites) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $output = & $browsy fetch $url 2>&1 | Out-String
    $ms = $sw.ElapsedMilliseconds
    $chars = $output.Length
    $lines = ($output -split "`n").Count
    Write-Output "${url}|${ms}|${lines}|${chars}"
}
