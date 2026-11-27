[Setup]
AppName=FFmpeg Studio
AppVersion=0.3.9
DefaultDirName={localappdata}\Programs\FFmpeg Studio
DefaultGroupName=FFmpeg Studio
PrivilegesRequired=lowest
SetupIconFile=assets\ffmpeg-studio.ico
UninstallDisplayIcon={app}\ffmpeg-gui.exe
OutputDir=.
OutputBaseFilename=FFmpeg-Studio-Windows-Setup
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Files]
Source: "FFmpeg-Studio-Windows\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "assets\ffmpeg-studio.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\FFmpeg Studio"; Filename: "{app}\ffmpeg-gui.exe"; IconFilename: "{app}\ffmpeg-gui.exe"
Name: "{autodesktop}\FFmpeg Studio"; Filename: "{app}\ffmpeg-gui.exe"; IconFilename: "{app}\ffmpeg-gui.exe"

[Run]
Filename: "{app}\ffmpeg-gui.exe"; Description: "Launch FFmpeg Studio"; Flags: nowait postinstall skipifsilent
