$browsy = 'C:\Users\Cade\projects\agentbrowser\target\release\browsy.exe'
$sites = @(
    'https://news.ycombinator.com',
    'https://github.com/login',
    'https://duckduckgo.com',
    'https://example.com',
    'https://httpbin.org/forms/post',
    'https://en.wikipedia.org/wiki/Main_Page',
    'https://www.bbc.com/news',
    'https://stackoverflow.com/questions',
    'https://craigslist.org'
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
