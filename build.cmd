@echo off
setlocal EnableExtensions DisableDelayedExpansion
cd /d "%~dp0"
chcp 65001 >nul
title Pealayer project-owned Windows build and packaging

where.exe pwsh.exe >nul 2>nul
if errorlevel 1 (
    echo [ERROR] PowerShell 7 was not found in PATH.
    exit /b 1
)

pwsh.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\package-windows.ps1" %*
exit /b %ERRORLEVEL%
