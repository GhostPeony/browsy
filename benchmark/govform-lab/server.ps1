param(
    [int]$Port = 8008
)

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

if (Get-Command python -ErrorAction SilentlyContinue) {
    python -m http.server $Port --bind 127.0.0.1
    exit 0
}

if (Get-Command py -ErrorAction SilentlyContinue) {
    py -m http.server $Port --bind 127.0.0.1
    exit 0
}

Write-Output "No Python runtime found. Install Python or run a simple static server."
exit 1
