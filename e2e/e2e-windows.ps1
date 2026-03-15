# E2E tests for Windows (requires Notepad running)
# Usage: .\e2e\e2e-windows.ps1 [-Bin path\to\binary] [-NotepadPid 1234]

param(
    [string]$Bin = ".\target\debug\agent-desktop.exe",
    [int]$NotepadPid = 0
)

$script:Pass = 0
$script:Fail = 0

function Pass($name) { Write-Host "  PASS: $name"; $script:Pass++ }
function Fail($name) { Write-Host "  FAIL: $name"; $script:Fail++ }

function Run-Test {
    param([string]$Name, [scriptblock]$Test)
    Write-Host "--- $Name ---"
    try {
        & $Test
        Pass $Name
    } catch {
        Write-Host "  Error: $_"
        Fail $Name
    }
}

function Assert-Match($output, $pattern, $msg) {
    if ($output -notmatch $pattern) { throw $msg }
}

function Assert-ExitFailure($msg) {
    if ($LASTEXITCODE -eq 0) { throw $msg }
}

# --- Helper to run a command and capture output including stderr ---
function Invoke-Bin {
    $ErrorActionPreference = 'Continue'
    $args_list = $args
    $output = & $Bin @args_list 2>&1 | Out-String
    return $output
}

# --- Accessibility tests (observe/interact — fully implemented) ---

Run-Test "observe --pid returns elements" {
    $output = Invoke-Bin observe --pid $NotepadPid
    Write-Host $output
    Assert-Match $output '<' "Expected XML output"
}

Run-Test "observe --pid --format json" {
    $output = Invoke-Bin observe --pid $NotepadPid --format json
    Write-Host ($output.Substring(0, [Math]::Min(500, $output.Length)))
    Assert-Match $output '[\[\{]' "Expected JSON output"
}

Run-Test "observe --pid --list-roles" {
    $output = Invoke-Bin observe --pid $NotepadPid --list-roles
    Write-Host $output
    Assert-Match $output '\d+' "Expected role counts"
}

Run-Test "observe --pid with query" {
    Invoke-Bin observe --pid $NotepadPid | Out-Null
    $output = Invoke-Bin observe --pid $NotepadPid -q 'window'
    Write-Host $output
    if ($output.Length -eq 0) { throw "Expected non-empty query result" }
}

Run-Test "interact press on element" {
    Invoke-Bin observe --pid $NotepadPid | Out-Null
    $ErrorActionPreference = 'Continue'
    $output = & $Bin interact --element 2 --action press 2>&1 | Out-String
    Write-Host "Interact output: $output"
    if ($output -match 'panic') { throw "interact panicked" }
}

# --- Tests for stubbed commands (should fail gracefully) ---

Run-Test "screenshot fails gracefully on Windows" {
    $output = Invoke-Bin screenshot --output C:\Temp\screen.png
    Write-Host $output
    Assert-ExitFailure "Expected screenshot to fail on Windows"
    Assert-Match $output 'not supported' "Expected 'not supported' message"
}

Run-Test "click at coordinates fails gracefully" {
    $output = Invoke-Bin click --x 100 --y 100
    Write-Host $output
    Assert-ExitFailure "Expected click to fail on Windows"
    Assert-Match $output 'not supported' "Expected 'not supported' message"
}

Run-Test "scroll fails gracefully" {
    $output = Invoke-Bin scroll --direction down
    Write-Host $output
    Assert-ExitFailure "Expected scroll to fail on Windows"
    Assert-Match $output 'not supported' "Expected 'not supported' message"
}

Run-Test "key press fails gracefully" {
    $output = Invoke-Bin key --name escape
    Write-Host $output
    Assert-ExitFailure "Expected key to fail on Windows"
    Assert-Match $output 'not supported' "Expected 'not supported' message"
}

Run-Test "type fails gracefully" {
    $output = Invoke-Bin type --text "hello"
    Write-Host $output
    Assert-ExitFailure "Expected type to fail on Windows"
    Assert-Match $output 'not supported' "Expected 'not supported' message"
}

Run-Test "read clipboard fails gracefully" {
    $output = Invoke-Bin read --clipboard
    Write-Host $output
    Assert-ExitFailure "Expected read clipboard to fail on Windows"
    Assert-Match $output 'not supported' "Expected 'not supported' message"
}

Run-Test "focus fails gracefully" {
    $output = Invoke-Bin focus --app Notepad
    Write-Host $output
    Assert-ExitFailure "Expected focus to fail on Windows"
    Assert-Match $output 'not supported' "Expected 'not supported' message"
}

# --- Observe without --app (not implemented for Windows) ---

Run-Test "observe without --app fails gracefully" {
    $output = Invoke-Bin observe
    Write-Host $output
    Assert-ExitFailure "Expected observe without --app to fail"
    Assert-Match $output 'not.*supported|error' "Expected informative error"
}

# --- CLI validation tests ---

Run-Test "observe with invalid app fails gracefully" {
    $output = Invoke-Bin observe --app NonExistentApp12345
    Write-Host $output
    Assert-ExitFailure "Expected failure for nonexistent app"
    Assert-Match $output 'not found|error' "Expected informative error"
}

Run-Test "click without element or coords fails" {
    $output = Invoke-Bin click
    Write-Host $output
    Assert-ExitFailure "Expected failure"
}

# --- Results ---

Write-Host ""
Write-Host "=== Results: $($script:Pass) passed, $($script:Fail) failed ==="
if ($script:Fail -gt 0) { exit 1 }
