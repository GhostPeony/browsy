$browsy = 'C:\Users\Cade\projects\agentbrowser\target\release\browsy.exe'
$logins = @(
    'https://github.com/login',
    'https://news.ycombinator.com/login',
    'https://accounts.google.com',
    'https://gitlab.com/users/sign_in',
    'https://bitbucket.org/account/signin/',
    'https://login.microsoftonline.com',
    'https://www.linkedin.com/login'
)
Write-Output "=== LOGIN DETECTION ==="
foreach ($url in $logins) {
    $json = & $browsy fetch $url --json 2>&1 | Out-String
    try {
        $p = $json | ConvertFrom-Json -ErrorAction Stop
        $pt = $p.page_type
        $acts = ""
        foreach ($a in $p.suggested_actions) {
            if ($a.action -eq 'Login') {
                $acts = "Login(u=$($a.username_id),p=$($a.password_id),s=$($a.submit_id))"
            }
        }
        if (-not $acts -and $p.suggested_actions.Count -gt 0) {
            $acts = ($p.suggested_actions | ForEach-Object { $_.action }) -join ","
        }
        if (-not $acts) { $acts = "none" }
        Write-Output "${url}|type=${pt}|actions=${acts}|els=$($p.els.Count)"
    } catch {
        Write-Output "${url}|PARSE_ERROR"
    }
}
