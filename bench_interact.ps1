$browsy = 'C:\Users\Cade\projects\agentbrowser\target\release\browsy.exe'

Write-Output "=== INTERACTION TESTS ==="
Write-Output ""

# Test 1: HN Login - fetch, identify fields, fill them
Write-Output "--- TEST 1: HN Login Flow ---"
$json = & $browsy fetch "https://news.ycombinator.com/login" --json 2>&1 | Out-String
$p = $json | ConvertFrom-Json -ErrorAction Stop
Write-Output "page_type: $($p.page_type)"
foreach ($a in $p.suggested_actions) {
    if ($a.action -eq 'Login') {
        Write-Output "Login action: u=$($a.username_id) p=$($a.password_id) s=$($a.submit_id)"
        # Now try typing into the username field
        $type_result = & $browsy type $a.username_id "testuser" --json 2>&1 | Out-String
        Write-Output "type(username): $($type_result.Substring(0, [Math]::Min(200, $type_result.Length)))"
        # Type into password field
        $type_result2 = & $browsy type $a.password_id "testpass" --json 2>&1 | Out-String
        Write-Output "type(password): $($type_result2.Substring(0, [Math]::Min(200, $type_result2.Length)))"
    }
}

Write-Output ""

# Test 2: DuckDuckGo Search - fetch, find search box, type query
Write-Output "--- TEST 2: DDG Search Flow ---"
$json = & $browsy fetch "https://duckduckgo.com" --json 2>&1 | Out-String
$p = $json | ConvertFrom-Json -ErrorAction Stop
Write-Output "page_type: $($p.page_type)"
Write-Output "elements: $($p.els.Count)"
# Find search input
foreach ($el in $p.els) {
    if ($el.tag -eq 'input' -and $el.name -eq 'q') {
        Write-Output "search input: id=$($el.id) name=$($el.name)"
        break
    }
}
foreach ($a in $p.suggested_actions) {
    Write-Output "action: $($a.action) input=$($a.input_id) submit=$($a.submit_id)"
}

Write-Output ""

# Test 3: httpbin form - fetch, fill fields, submit
Write-Output "--- TEST 3: httpbin Form Fill ---"
$json = & $browsy fetch "https://httpbin.org/forms/post" --json 2>&1 | Out-String
$p = $json | ConvertFrom-Json -ErrorAction Stop
Write-Output "page_type: $($p.page_type)"
Write-Output "elements: $($p.els.Count)"
foreach ($el in $p.els) {
    if ($el.tag -match 'input|select|textarea') {
        Write-Output "  field: id=$($el.id) tag=$($el.tag) name=$($el.name) type=$($el.input_type)"
    }
}
foreach ($a in $p.suggested_actions) {
    Write-Output "action: $($a.action)"
}

Write-Output ""

# Test 4: GitHub login - full interaction capability check
Write-Output "--- TEST 4: GitHub Login Fields ---"
$json = & $browsy fetch "https://github.com/login" --json 2>&1 | Out-String
$p = $json | ConvertFrom-Json -ErrorAction Stop
Write-Output "page_type: $($p.page_type)"
foreach ($a in $p.suggested_actions) {
    if ($a.action -eq 'Login') {
        Write-Output "Login: u=$($a.username_id) p=$($a.password_id) s=$($a.submit_id)"
        # Verify the IDs point to real elements
        $u_el = $p.els | Where-Object { $_.id -eq $a.username_id }
        $p_el = $p.els | Where-Object { $_.id -eq $a.password_id }
        $s_el = $p.els | Where-Object { $_.id -eq $a.submit_id }
        Write-Output "  username el: tag=$($u_el.tag) name=$($u_el.name) type=$($u_el.input_type)"
        Write-Output "  password el: tag=$($p_el.tag) name=$($p_el.name) type=$($p_el.input_type)"
        Write-Output "  submit el: tag=$($s_el.tag) text=$($s_el.text)"
    }
}
