# PowerShell Script to Delete GitHub Actions Workflow Runs
# Requires: GitHub CLI (gh)
# 
# Run this from any Windows/macOS/Linux machine with PowerShell
# ============================================================================

param(
    [string]$Repo = "Nespartious/Fortify",
    [int]$BatchSize = 50,
    [int]$MaxParallel = 10,
    [switch]$Force,
    [switch]$SkipLogin
)

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " GitHub Actions Workflow Run Cleaner" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# ============================================================================
# STEP 1: Check if GitHub CLI is installed
# ============================================================================
Write-Host "[1/4] Checking for GitHub CLI..." -ForegroundColor Yellow

$ghInstalled = Get-Command gh -ErrorAction SilentlyContinue

if (-not $ghInstalled) {
    Write-Host "GitHub CLI (gh) not found. Installing..." -ForegroundColor Red
    
    if ($IsWindows -or $env:OS -match "Windows") {
        # Windows: Use winget or chocolatey
        Write-Host "Attempting installation via winget..." -ForegroundColor Yellow
        try {
            winget install --id GitHub.cli -e --source winget
        } catch {
            Write-Host "winget failed. Trying chocolatey..." -ForegroundColor Yellow
            try {
                choco install gh -y
            } catch {
                Write-Host "ERROR: Could not install gh CLI automatically." -ForegroundColor Red
                Write-Host "Please install manually from: https://cli.github.com/" -ForegroundColor Red
                Write-Host ""
                Write-Host "Installation options:" -ForegroundColor White
                Write-Host "  - winget install --id GitHub.cli" -ForegroundColor Gray
                Write-Host "  - choco install gh" -ForegroundColor Gray
                Write-Host "  - scoop install gh" -ForegroundColor Gray
                Write-Host "  - Download from https://github.com/cli/cli/releases" -ForegroundColor Gray
                exit 1
            }
        }
    } elseif ($IsMacOS) {
        # macOS: Use homebrew
        Write-Host "Attempting installation via Homebrew..." -ForegroundColor Yellow
        brew install gh
    } else {
        # Linux
        Write-Host "Attempting installation for Linux..." -ForegroundColor Yellow
        # Try various package managers
        if (Get-Command apt -ErrorAction SilentlyContinue) {
            sudo apt update
            sudo apt install gh -y
        } elseif (Get-Command dnf -ErrorAction SilentlyContinue) {
            sudo dnf install gh -y
        } elseif (Get-Command pacman -ErrorAction SilentlyContinue) {
            sudo pacman -S github-cli --noconfirm
        } else {
            Write-Host "ERROR: Could not detect package manager." -ForegroundColor Red
            Write-Host "Please install gh manually: https://cli.github.com/" -ForegroundColor Red
            exit 1
        }
    }
    
    # Verify installation
    $ghInstalled = Get-Command gh -ErrorAction SilentlyContinue
    if (-not $ghInstalled) {
        Write-Host "ERROR: gh CLI installation failed." -ForegroundColor Red
        exit 1
    }
}

Write-Host "  GitHub CLI found: $(gh --version | Select-Object -First 1)" -ForegroundColor Green

# ============================================================================
# STEP 2: Authenticate with GitHub
# ============================================================================
Write-Host ""
Write-Host "[2/4] Checking GitHub authentication..." -ForegroundColor Yellow

if (-not $SkipLogin) {
    $authStatus = gh auth status 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  Not logged in. Starting authentication..." -ForegroundColor Yellow
        Write-Host ""
        Write-Host "  You'll need to authenticate with GitHub." -ForegroundColor White
        Write-Host "  Required scopes: repo, workflow" -ForegroundColor Gray
        Write-Host ""
        
        gh auth login --scopes repo,workflow
        
        if ($LASTEXITCODE -ne 0) {
            Write-Host "ERROR: Authentication failed." -ForegroundColor Red
            exit 1
        }
    } else {
        Write-Host "  Already authenticated!" -ForegroundColor Green
    }
}

# ============================================================================
# STEP 3: Count workflow runs
# ============================================================================
Write-Host ""
Write-Host "[3/4] Fetching workflow runs for $Repo..." -ForegroundColor Yellow

try {
    $runsJson = gh run list --repo $Repo --limit 500 --json databaseId,status,conclusion,name,createdAt 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Failed to fetch runs: $runsJson" -ForegroundColor Red
        exit 1
    }
    
    $runs = $runsJson | ConvertFrom-Json
    $totalRuns = $runs.Count
    
    Write-Host "  Found $totalRuns workflow runs" -ForegroundColor Green
    
    if ($totalRuns -eq 0) {
        Write-Host ""
        Write-Host "No workflow runs to delete!" -ForegroundColor Green
        exit 0
    }
    
    # Show summary
    $completed = ($runs | Where-Object { $_.status -eq "completed" }).Count
    $inProgress = ($runs | Where-Object { $_.status -eq "in_progress" }).Count
    $queued = ($runs | Where-Object { $_.status -eq "queued" }).Count
    
    Write-Host ""
    Write-Host "  Summary:" -ForegroundColor White
    Write-Host "    Completed:   $completed" -ForegroundColor Gray
    Write-Host "    In Progress: $inProgress" -ForegroundColor Gray
    Write-Host "    Queued:      $queued" -ForegroundColor Gray
    
} catch {
    Write-Host "ERROR: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# ============================================================================
# STEP 4: Delete workflow runs
# ============================================================================
Write-Host ""
Write-Host "[4/4] Deleting workflow runs..." -ForegroundColor Yellow

if (-not $Force) {
    Write-Host ""
    $confirm = Read-Host "  Delete all $totalRuns workflow runs? (y/N)"
    if ($confirm -notmatch "^[yY]") {
        Write-Host "  Cancelled." -ForegroundColor Yellow
        exit 0
    }
}

$deleted = 0
$failed = 0
$runIds = $runs | ForEach-Object { $_.databaseId }

# Process in batches
for ($i = 0; $i -lt $runIds.Count; $i += $BatchSize) {
    $batch = $runIds[$i..([Math]::Min($i + $BatchSize - 1, $runIds.Count - 1))]
    $batchNum = [Math]::Floor($i / $BatchSize) + 1
    $totalBatches = [Math]::Ceiling($runIds.Count / $BatchSize)
    
    Write-Host "  Batch $batchNum/$totalBatches (runs $($i + 1)-$([Math]::Min($i + $BatchSize, $runIds.Count)))..." -ForegroundColor Gray
    
    # Delete in parallel using PowerShell jobs
    $jobs = @()
    foreach ($runId in $batch) {
        $jobs += Start-Job -ScriptBlock {
            param($repo, $id)
            gh run delete $id --repo $repo 2>&1
            return $LASTEXITCODE
        } -ArgumentList $Repo, $runId
        
        # Limit parallel jobs
        while (($jobs | Where-Object { $_.State -eq 'Running' }).Count -ge $MaxParallel) {
            Start-Sleep -Milliseconds 100
        }
    }
    
    # Wait for batch to complete
    $results = $jobs | Wait-Job | Receive-Job
    $jobs | Remove-Job
    
    $batchDeleted = ($results | Where-Object { $_ -eq 0 }).Count
    $batchFailed = $batch.Count - $batchDeleted
    
    $deleted += $batchDeleted
    $failed += $batchFailed
    
    # Progress
    $progress = [Math]::Round(($deleted + $failed) / $totalRuns * 100)
    Write-Host "    Deleted: $deleted | Failed: $failed | Progress: $progress%" -ForegroundColor Gray
    
    # Rate limit protection - pause between batches
    if ($i + $BatchSize -lt $runIds.Count) {
        Start-Sleep -Seconds 1
    }
}

# ============================================================================
# SUMMARY
# ============================================================================
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host " Deletion Complete!" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Total runs:    $totalRuns" -ForegroundColor White
Write-Host "  Deleted:       $deleted" -ForegroundColor Green
Write-Host "  Failed:        $failed" -ForegroundColor $(if ($failed -gt 0) { "Red" } else { "Green" })
Write-Host ""

if ($failed -gt 0) {
    Write-Host "Some deletions failed. This may be due to:" -ForegroundColor Yellow
    Write-Host "  - Rate limiting (wait and retry)" -ForegroundColor Gray
    Write-Host "  - Runs still in progress" -ForegroundColor Gray
    Write-Host "  - Permission issues" -ForegroundColor Gray
}
