# Set Working Directory
Split-Path $MyInvocation.MyCommand.Path | Push-Location
[Environment]::CurrentDirectory = $PWD

Remove-Item "$env:RELOADEDIIMODS/riri.imguihookex/*" -Force -Recurse
dotnet publish "./riri.imguihookex.csproj" -c Release -o "$env:RELOADEDIIMODS/riri.imguihookex" /p:OutputPath="./bin/Release" /p:ReloadedILLink="true"

# Restore Working Directory
Pop-Location