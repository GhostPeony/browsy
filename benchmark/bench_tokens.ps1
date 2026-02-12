$browsy = 'C:\Users\Cade\projects\agentbrowser\target\release\browsy.exe'
$sites = @(
    'https://news.ycombinator.com',
    'https://github.com/login',
    'https://duckduckgo.com',
    'https://example.com',
    'https://httpbin.org/forms/post',
    'https://en.wikipedia.org/wiki/Main_Page'
)
Write-Output "=== BROWSY COMPACT OUTPUT ==="
Write-Output "SITE|CHARS|LINES|WORDS"
foreach ($url in $sites) {
    $output = & $browsy fetch $url 2>&1 | Out-String
    $chars = $output.Length
    $lines = ($output -split "`n").Count
    $words = ($output -split '\s+').Count
    Write-Output "${url}|${chars}|${lines}|${words}"
}
Write-Output ""
Write-Output "=== BROWSY JSON OUTPUT ==="
Write-Output "SITE|JSON_CHARS|ELS|PAGE_TYPE"
foreach ($url in $sites) {
    $json = & $browsy fetch $url --json 2>&1 | Out-String
    try {
        $p = $json | ConvertFrom-Json -ErrorAction Stop
        $jchars = $json.Length
        $els = $p.els.Count
        $pt = $p.page_type
        Write-Output "${url}|${jchars}|${els}|${pt}"
    } catch {
        Write-Output "${url}|PARSE_ERROR"
    }
}
