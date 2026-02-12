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

Write-Output "SITE|MS|LINES|CHARS|STATUS|NOTE"
foreach ($url in $sites) {
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $output = & $browsy fetch $url 2>&1 | Out-String
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

    Write-Output "${url}|${ms}|${lines}|${chars}|${status}|${note}"
}
