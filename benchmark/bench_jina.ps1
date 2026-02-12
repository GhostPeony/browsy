$sites = @(
    @{url='https://news.ycombinator.com'; name='HN'},
    @{url='https://github.com/login'; name='GitHub Login'},
    @{url='https://duckduckgo.com'; name='DuckDuckGo'},
    @{url='https://example.com'; name='Example'}
)
Write-Output "=== JINA READER ==="
Write-Output "SITE|CHARS|LINES|WORDS"
foreach ($s in $sites) {
    try {
        $resp = Invoke-WebRequest -Uri "https://r.jina.ai/$($s.url)" -Headers @{'Accept'='text/plain'} -TimeoutSec 15 -ErrorAction Stop
        $body = $resp.Content
        $chars = $body.Length
        $lines = ($body -split "`n").Count
        $words = ($body -split '\s+').Count
        Write-Output "$($s.name)|${chars}|${lines}|${words}"
    } catch {
        Write-Output "$($s.name)|ERROR|$($_.Exception.Message)"
    }
}
