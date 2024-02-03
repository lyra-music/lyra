# For starting the lavalink server (Windows)

$R = [consolecolor]::Red
$Y = [consolecolor]::Yellow
$B = [consolecolor]::Blue
$r = [consolecolor]::Gray

$ENV_F = ".env"
$APP_F = "application.yml"
$_APP_F = "..application.yml"
$LVLJ_F = "Lavalink.jar"
$JAVA_F = "java"
$LVLL_D = "lavalink"

$ALL_F = $true

if (-not (Test-Path $ENV_F)) {
    Write-Host ("error: {0} not found in {1}" -f $Y, $B) -ForegroundColor $R
    $ALL_F = $false
}

if (-not (Test-Path $LVLL_D -PathType Container)) {
    Write-Host ("error: {0} not found in {1}" -f $Y, $B) -ForegroundColor $R
    $ALL_F = $false
}

foreach ($_f in $APP_F, $LVLJ_F, $JAVA_F) {
    if (-not (Test-Path (Join-Path $LVLL_D $_f))) {
        Write-Host ("error: {0} not found in {1}{2}" -f $Y, $B, $LVLL_D) -ForegroundColor $R
        $ALL_F = $false
    }
}

if (-not $ALL_F) {
    Write-Host "Please change the working directory and/or create all the missing files then rerun this script"
    exit
}

Set-Location $LVLL_D

Copy-Item $APP_F $_APP_F
$env:JAVA_OPTS = Get-Content "../$ENV_F" | Out-String
(Get-Content $_APP_F) -replace "`$`{(\w+)}", { $matches[$_.Groups[1].Value] } | Set-Content $APP_F

Start-Process $JAVA_F -ArgumentList "-jar", $LVLJ_F

Remove-Item $APP_F
Move-Item $_APP_F $APP_F

Set-Location ..

# Note: PowerShell doesn't have an equivalent to 'sudo chattr', and it might not be necessary on Windows.
# Uncomment the related lines if needed in a Linux environment.
