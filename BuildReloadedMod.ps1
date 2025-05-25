param (
    [System.Boolean] $IsDebug = $False
)

[System.Boolean] $global:is_pushed = $False

function GoToRelativeFolder {
    param (
        [string] $ChildPath
    )
    if ($global:is_pushed) { Pop-Location }
    [IO.Path]::Combine((Get-Location).ToString(), $ChildPath) | Push-Location
    $global:is_pushed = $True
}

function GoToFolder {
    param (
        [string] $Path
    )
    if ($global:is_pushed) { Pop-Location }
    $Path | Push-Location
    $global:is_pushed = $True
}

function GetNonNullEnvironmentVariable {
    param (
        [string] $EnvVariable
    )
    $TryEnvValue = [System.Environment]::GetEnvironmentVariable($EnvVariable)
    if ([System.String]::IsNullOrEmpty($TryEnvValue)) {
        Write-Error "No value was provided for the environmental variable ${EnvVariable}. 
This should be set before executing BuildReloadedMod"
    } else {
        $TryEnvValue
    }
}

function SetEnvironmentVariableIfNull {
    param (
        [string] $EnvVariable,
        [string] $EnvValue
    )
    $TryEnvValue = [System.Environment]::GetEnvironmentVariable($EnvVariable)
    if ([System.String]::IsNullOrEmpty($TryEnvValue)) {
        [System.Environment]::SetEnvironmentVariable($EnvVariable, $EnvValue)
    }
}

function GetNameWithUnderscores {
    param (
        [string] $Name
    )
    $Name.Replace("-", "_")
}

[string] $global:GLOBALS_CRATE = "riri-imgui-hook-globals"
[string] $global:RELOADED_CRATE = "riri-imgui-hook-reloaded"
[string] $global:RELOADED_ENTRYPOINT = "riri.imguihookex"
[string] $global:LIB_CRATE = "riri-imgui-hook"
# Always will be given the game's compiled using MSVC
[string] $global:TARGET = "x86_64-pc-windows-msvc"

function GetRustCrateTargetDirectory {
    param (
        [string] $Path
    )
    $ProfileFolder = if ($IsDebug) { "release-debug" } else { "release" }
    GoToFolder -Path ([IO.Path]::Combine($Path, "target", $global:TARGET, $ProfileFolder))
}

function BuildRustCrate {
    param (
        [string] $FriendlyName,
        [string] $BuildStd,
        [string] $BuildStdFeatures,
        [string] $CrateType
    )
    $RustProfile = if ($IsDebug) { "--profile=slow-debug" } else { "--profile=release" }
    cargo +nightly rustc --lib $RustProfile -Z build-std=$BuildStd -Z build-std-features=$BuildStdFeatures --crate-type $CrateType --target $global:TARGET
    # cargo +nightly rustc -vv --lib $RustProfile -Z build-std=$BuildStd -Z build-std-features=$BuildStdFeatures --crate-type $CrateType --target $global:TARGET
    if (!$?) {
        Write-Error "Failed to build the Rust crate ${FriendlyName}"
    }
}

function BuildCsharpProject {
    param (
        [string] $FriendlyName,
        [string] $ProjectName
    )
    dotnet build $ProjectName -v q -c Debug
    if (!$?) {
        Write-Error "Failed to build the C# project ${FriendlyName}"
    }
}

function CopyOutputArtifacts {
    param (
        [string] $CrateName,
        [string] $SourceDirectory,
        [string] $TargetDirectory
    )
    $UnderscoreName = GetNameWithUnderscores $CrateName
    Copy-Item ([IO.Path]::Combine($SourceDirectory, "${UnderscoreName}.dll")) -Destination $TargetDirectory
    Copy-Item ([IO.Path]::Combine($SourceDirectory, "${UnderscoreName}.dll.lib")) -Destination $TargetDirectory
    Copy-Item ([IO.Path]::Combine($SourceDirectory, "${UnderscoreName}.dll.exp")) -Destination $TargetDirectory
    if ($IsDebug) {
        Copy-Item ([IO.Path]::Combine($SourceDirectory, "${UnderscoreName}.pdb")) -Destination $TargetDirectory
    }
}

# Set working directory
Split-Path $MyInvocation.MyCommand.Path | Push-Location
[Environment]::CurrentDirectory = $PWD
$BASE_PATH = (Get-Location).ToString();
[System.Environment]::SetEnvironmentVariable("RUST_BACKTRACE", 1)
[System.Environment]::SetEnvironmentVariable("RUSTFLAGS", "-C panic=abort -C lto=fat -C embed-bitcode=yes -C target-feature=+avx2")

$RELOADED_MOD_DIRECTORY = [IO.Path]::Combine((GetNonNullEnvironmentVariable -EnvVariable RELOADEDIIMODS), $global:RELOADED_ENTRYPOINT)

# build OpenGFD globals as DLL
GoToFolder -Path ([IO.Path]::Combine($BASE_PATH, $global:GLOBALS_CRATE))
# We need to call it's build script to produce functions for each defined global
# so that they can be linked into any mod using OpenGFD

$GLOBAL_FILE = ([IO.Path]::Combine($BASE_PATH, $global:LIB_CRATE, "src", "globals.rs"))
$RELOADED_GLB = ([IO.Path]::Combine($BASE_PATH, $global:RELOADED_CRATE, "src", "globals.rs"))
BuildRustCrate -FriendlyName $global:GFD_GLOBALS_CRATE -BuildStd "std,panic_abort" -BuildStdFeatures "panic_immediate_abort" -CrateType "cdylib"
Copy-Item ([IO.Path]::Combine((Get-Location).ToString(), "middata", "self.rs")) -Destination $GLOBAL_FILE -Force
Copy-Item ([IO.Path]::Combine((Get-Location).ToString(), "middata", "ext.rs")) -Destination $RELOADED_GLB -Force

# build OpenGFD Reloaded project (Rust portion)
GoToFolder -Path ([IO.Path]::Combine($BASE_PATH, $global:RELOADED_CRATE))
BuildRustCrate -FriendlyName $global:RELOADED_CRATE -BuildStd "std,panic_abort" -BuildStdFeatures "panic_immediate_abort" -CrateType "cdylib"

# build OpenGFD Reloaded project (C# portion)
GoToFolder -Path ([IO.Path]::Combine($BASE_PATH, $global:RELOADED_ENTRYPOINT))
BuildCsharpProject -FriendlyName $global:RELOADED_ENTRYPOINT -ProjectName "${RELOADED_ENTRYPOINT}.csproj"

# copy files from our Rust project folder into the Reloaded mod
GetRustCrateTargetDirectory -Path $BASE_PATH
CopyOutputArtifacts -CrateName $global:RELOADED_CRATE -SourceDirectory (Get-Location).ToString() -TargetDirectory $RELOADED_MOD_DIRECTORY
CopyOutputArtifacts -CrateName $global:GLOBALS_CRATE -SourceDirectory (Get-Location).ToString() -TargetDirectory $RELOADED_MOD_DIRECTORY

# Restore Working Directory
Pop-Location
