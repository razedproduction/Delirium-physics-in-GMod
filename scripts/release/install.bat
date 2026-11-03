@echo off
rem DMS installer for players: double-click it in the unpacked release (DMS-<version>-win64\), Garry's Mod closed.
rem
rem What goes where (install.md):
rem   bin\vphysics.dll        -> GarrysMod\bin\win64\vphysics.dll (Steam's own kept once as vphysics.dll.original)
rem   bin\gmsv_dms_win64.dll  -> GarrysMod\garrysmod\lua\bin
rem   your GTA IV folder      -> GarrysMod\garrysmod\data\dms\game_dir.txt (the path only), then bin\dms-homer.exe
rem                              makes garrysmod\models\dms\homer.mdl from your copy of the game
rem The Lua addon isn't in the release: it's the Workshop's (WORKSHOP below). A garrysmod\addons\dms left by an older
rem release is removed, so only the Workshop's Lua runs. Nothing of GTA IV is copied anywhere, nothing leaves your machine.
rem
rem No ( ) blocks with paths in them: "Program Files (x86)" would end a block; one-line ifs and gotos only.

setlocal EnableExtensions DisableDelayedExpansion
title DMS installer
cd /d "%~dp0"
set "HERE=%~dp0"
rem scripts\package.ps1 puts the release's version here.
set "VERSION=@VERSION@"
set "WORKSHOP_ID=3829811829"
set "WORKSHOP=https://steamcommunity.com/sharedfiles/filedetails/?id=%WORKSHOP_ID%"
rem UTF-8, so a GTA IV folder with non-Latin letters reaches game_dir.txt as DMS reads it; the old code page at the end.
rem chcp reads stdin away when it's redirected: <nul.
set "OLDCP="
for /f "tokens=2 delims=:" %%C in ('chcp ^<nul') do set "OLDCP=%%C"
chcp 65001 <nul >nul
rem Answers asked again at most this many times in all (a closed input would ask for ever).
set "TRIES=0"

echo ==========================================================
echo   DMS %VERSION% for Garry's Mod (64-bit, Windows)
echo ==========================================================
echo.
echo DMS needs two things from you:
echo   - the DMS addon from the Steam Workshop (the Lua part):
echo     %WORKSHOP%
echo   - your own copy of GTA IV, to make Homer's model from it.
echo.

:ask_action
set /a TRIES+=1
if %TRIES% GTR 10 goto no_answer
echo   1. Install or update DMS and make Homer's model (first install, or a release that asks for it)
echo   2. Install or update DMS only (Homer's model stays as it is)
echo   3. Make Homer's model only
echo   4. Uninstall DMS
echo.
set "ACTION="
set /p "ACTION=Choose 1, 2, 3 or 4: "
set "DO_INSTALL=0"
set "DO_MODEL=0"
if "%ACTION%"=="1" set "DO_INSTALL=1"& set "DO_MODEL=1"& goto get_gmod
if "%ACTION%"=="2" set "DO_INSTALL=1"& goto get_gmod
if "%ACTION%"=="3" set "DO_MODEL=1"& goto get_gmod
if "%ACTION%"=="4" goto get_gmod
echo Type a number from 1 to 4.
echo.
goto ask_action

rem ---- Garry's Mod: Steam's own library, then the other ones (libraryfolders.vdf), then asked.
:get_gmod
echo.
set "GMOD="
set "STEAM="
for /f "tokens=2,*" %%A in ('reg query "HKCU\Software\Valve\Steam" /v SteamPath 2^>nul ^| find /i "SteamPath"') do set "STEAM=%%B"
if not defined STEAM goto ask_gmod
set "STEAM=%STEAM:/=\%"
call :try_library "%STEAM%"
set "VDF=%STEAM%\steamapps\libraryfolders.vdf"
if not defined GMOD if exist "%VDF%" for /f "usebackq tokens=1,*" %%A in ("%VDF%") do if /i "%%~A"=="path" call :try_library %%B
if not defined GMOD goto ask_gmod
echo Garry's Mod found: %GMOD%
set "ANSWER="
set /p "ANSWER=Use this folder? [Y/n]: "
if /i "%ANSWER%"=="n" goto ask_gmod
goto check_gmod

:ask_gmod
set /a TRIES+=1
if %TRIES% GTR 10 goto no_answer
echo Garry's Mod's folder (the one with garrysmod\ and bin\ in it; you can drag it onto this window),
echo for example D:\SteamLibrary\steamapps\common\GarrysMod
set "GMOD="
set /p "GMOD=Garry's Mod folder: "
if not defined GMOD goto ask_gmod
call :clean_path GMOD

:check_gmod
if not exist "%GMOD%\garrysmod\" echo ERROR: no garrysmod folder in %GMOD%& goto ask_gmod
set "WIN64=%GMOD%\bin\win64"
set "GAMEDATA=%GMOD%\garrysmod"
set "ORIGINAL=%WIN64%\vphysics.dll.original"
if not exist "%WIN64%\vphysics.dll" goto no_x64
tasklist /fo csv /nh 2>nul | findstr /i /b /c:"\"gmod" /c:"\"hl2.exe" >nul
if not errorlevel 1 echo ERROR: Garry's Mod is running: close it first (its vphysics.dll can't be replaced while it runs).& goto fail
if "%ACTION%"=="4" goto uninstall
if "%DO_INSTALL%"=="1" goto install
goto model

:no_x64
echo ERROR: %WIN64%\vphysics.dll isn't there: DMS needs 64-bit Garry's Mod (its bin\win64 folder);
echo let Steam update the game, or check this is the GarrysMod folder.
goto fail

rem ---- The fork and the module.
:install
echo.
for %%F in (vphysics.dll gmsv_dms_win64.dll dms-homer.exe) do if not exist "%HERE%bin\%%F" echo ERROR: bin\%%F is missing: unpack the whole zip.& goto fail
rem Only the first install keeps what is there: later ones would back up the fork itself.
if exist "%ORIGINAL%" goto copy_fork
copy /y "%WIN64%\vphysics.dll" "%ORIGINAL%" >nul
if errorlevel 1 echo ERROR: can't back up %WIN64%\vphysics.dll (try "Run as administrator").& goto fail
:copy_fork
copy /y "%HERE%bin\vphysics.dll" "%WIN64%\vphysics.dll" >nul
if errorlevel 1 echo ERROR: can't write %WIN64%\vphysics.dll (try "Run as administrator").& goto fail
echo -^> %WIN64%\vphysics.dll (VPhysics-Box3D with IPhysicsNM; Steam's own: vphysics.dll.original)
if not exist "%GAMEDATA%\lua\bin\" mkdir "%GAMEDATA%\lua\bin"
copy /y "%HERE%bin\gmsv_dms_win64.dll" "%GAMEDATA%\lua\bin\gmsv_dms_win64.dll" >nul
if errorlevel 1 echo ERROR: can't write %GAMEDATA%\lua\bin\gmsv_dms_win64.dll.& goto fail
echo -^> %GAMEDATA%\lua\bin\gmsv_dms_win64.dll
rem An older release's Lua (0.1.0 put it here) would run beside the Workshop's.
if not exist "%GAMEDATA%\addons\dms\" goto workshop
rmdir /s /q "%GAMEDATA%\addons\dms"
echo removed %GAMEDATA%\addons\dms (an older release's Lua: the addon is the Workshop's now)

:workshop
rem The Workshop's downloads are in the library's steamapps\workshop\content\4000 (Garry's Mod is app 4000).
if exist "%GMOD%\..\..\workshop\content\4000\%WORKSHOP_ID%\" echo The DMS addon is subscribed on the Workshop.& goto after_install
echo.
echo The DMS addon (its Lua) isn't downloaded from the Workshop yet: subscribe to it, or DMS won't start.
echo   %WORKSHOP%
set "ANSWER="
set /p "ANSWER=Open its Workshop page in Steam? [Y/n]: "
if /i not "%ANSWER%"=="n" start "" "steam://url/CommunityFilePage/%WORKSHOP_ID%"

:after_install
if "%DO_MODEL%"=="1" goto model
if exist "%GAMEDATA%\models\dms\homer.mdl" goto done
echo.
echo Homer's model isn't made yet: run install.bat again and choose 3.
goto done

rem ---- Homer's model, by dms-homer from the player's GTA IV.
:model
echo.
if not exist "%HERE%bin\dms-homer.exe" echo ERROR: bin\dms-homer.exe is missing: unpack the whole zip.& goto fail
set "GTA="
if exist "%GAMEDATA%\data\dms\game_dir.txt" set /p GTA=<"%GAMEDATA%\data\dms\game_dir.txt"
if not defined GTA goto ask_gta
echo GTA IV folder from the last time: %GTA%
set "ANSWER="
set /p "ANSWER=Use this folder? [Y/n]: "
if /i "%ANSWER%"=="n" goto ask_gta
goto check_gta

:ask_gta
set /a TRIES+=1
if %TRIES% GTR 10 goto no_answer
echo Your GTA IV folder: the PC game's (with GTAIV.exe) or an Xbox 360 copy's (with default.xex),
echo for example D:\Games\Grand Theft Auto IV
set "GTA="
set /p "GTA=GTA IV folder: "
if not defined GTA goto ask_gta
call :clean_path GTA

:check_gta
if exist "%GTA%\GTAIV.exe" goto gta_ok
if exist "%GTA%\default.xex" goto gta_ok
echo ERROR: %GTA% isn't a GTA IV folder (no GTAIV.exe or default.xex in it).
goto ask_gta
:gta_ok
if not exist "%GAMEDATA%\data\dms\" mkdir "%GAMEDATA%\data\dms"
> "%GAMEDATA%\data\dms\game_dir.txt" echo %GTA%
echo -^> %GAMEDATA%\data\dms\game_dir.txt (DMS reads NM's data from %GTA%; dms_game_dir overrides it)
echo.
echo Making Homer's model (a minute or so)...
"%HERE%bin\dms-homer.exe" --game "%GTA%" --gmod "%GMOD%" <nul
if errorlevel 1 echo ERROR: dms-homer failed (its messages are above).& goto fail
goto done

rem ---- Steam's vphysics.dll back, the module out; the Workshop's addon is unsubscribed in Steam.
:uninstall
echo.
echo Steam's vphysics.dll goes back and the DMS module is removed. Kept (yours, made from your GTA IV):
echo garrysmod\models\dms, garrysmod\materials\models\dms, garrysmod\data\dms.
set "ANSWER="
set /p "ANSWER=Uninstall DMS? [y/N]: "
if /i not "%ANSWER%"=="y" echo Nothing changed.& goto done
if not exist "%ORIGINAL%" goto no_original
copy /y "%ORIGINAL%" "%WIN64%\vphysics.dll" >nul
if errorlevel 1 echo ERROR: can't write %WIN64%\vphysics.dll (try "Run as administrator").& goto fail
del /q "%ORIGINAL%"
echo -^> %WIN64%\vphysics.dll: Steam's own again
goto remove_module
:no_original
echo No vphysics.dll.original: vphysics.dll left as it is (Steam's "Verify integrity of game files" puts its own back).
:remove_module
if exist "%GAMEDATA%\lua\bin\gmsv_dms_win64.dll" del /q "%GAMEDATA%\lua\bin\gmsv_dms_win64.dll"
echo removed %GAMEDATA%\lua\bin\gmsv_dms_win64.dll
if exist "%GAMEDATA%\addons\dms\" rmdir /s /q "%GAMEDATA%\addons\dms"
echo Unsubscribe from the DMS addon on the Workshop to remove its Lua too: %WORKSHOP%
set "CODE=0"
goto quit

:done
echo.
echo Done. In Garry's Mod (a local game, 64-bit): dms_homer in the console spawns Homer,
echo dms_menu opens the DMS window.
set "CODE=0"
goto quit

:no_answer
echo.
echo No answer: run install.bat again.
set "CODE=1"
goto quit

:fail
echo.
echo Stopped: nothing after the error was done.
set "CODE=1"

:quit
echo.
pause
if defined OLDCP chcp %OLDCP% >nul
exit /b %CODE%

rem ---- Helpers.

rem A Steam library (its path as libraryfolders.vdf writes it, \\ for \): Garry's Mod there, if it's the first found.
:try_library
if defined GMOD exit /b 0
set "LIB=%~1"
set "LIB=%LIB:\\=\%"
if exist "%LIB%\steamapps\common\GarrysMod\garrysmod\" set "GMOD=%LIB%\steamapps\common\GarrysMod"
exit /b 0

rem A typed or dragged path: no quotes, no trailing \ (it would escape the closing quote of "%X%" for an exe).
:clean_path
call set "P=%%%1%%"
set "P=%P:"=%"
if "%P:~-1%"=="\" set "P=%P:~0,-1%"
set "%1=%P%"
exit /b 0
