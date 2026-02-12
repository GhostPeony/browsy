$browsy = 'C:\Users\Cade\projects\agentbrowser\target\release\browsy.exe'
$pages = @(
    @{url='https://github.com/login'; expected='Login'},
    @{url='https://news.ycombinator.com/login'; expected='Login'},
    @{url='https://duckduckgo.com'; expected='Search'},
    @{url='https://news.ycombinator.com'; expected='List'},
    @{url='https://stackoverflow.com/questions'; expected='List'},
    @{url='https://httpbin.org/forms/post'; expected='Form'},
    @{url='https://example.com'; expected='Other'},
    @{url='https://craigslist.org'; expected='List'},
    @{url='https://www.bbc.com/news'; expected='List'},
    @{url='https://www.python.org'; expected='Search'}
)
Write-Output "=== PAGE TYPE DETECTION ==="
$correct = 0; $total = 0
foreach ($pg in $pages) {
    $total++
    $json = & $browsy fetch $pg.url --json 2>&1 | Out-String
    try {
        $p = $json | ConvertFrom-Json -ErrorAction Stop
        $detected = $p.page_type
        $match = if ($pg.expected -eq $detected) { "PASS" } else { "MISS" }
        if ($match -eq "PASS") { $correct++ }
        Write-Output "${match}|$($pg.url)|expected=$($pg.expected)|detected=${detected}|els=$($p.els.Count)"
    } catch {
        Write-Output "ERROR|$($pg.url)|parse failed"
    }
}
Write-Output "SCORE|${correct}/${total}"
