param(
    [string]$Url = "https://httpbin.org/forms/post"
)

$env:NPM_CONFIG_YES = "true"
npx --yes agent-browser open $Url 2>&1 | Out-Null
npx --yes agent-browser snapshot -i 2 2>&1 | Out-Null
