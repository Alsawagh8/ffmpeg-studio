[Setup]
AppName=FFmpeg Studio
AppVersion=0.3.6
DefaultDirName={autopf}\FFmpeg Studio
DefaultGroupName=FFmpeg Studio
OutputDir=.
OutputBaseFilename=FFmpeg-Studio-Windows-Setup
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Files]
Source: "FFmpeg-Studio-Windows\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\FFmpeg Studio"; Filename: "{app}\ffmpeg-gui.exe"
Name: "{autodesktop}\FFmpeg Studio"; Filename: "{app}\ffmpeg-gui.exe"

[Run]
Filename: "{app}\ffmpeg-gui.exe"; Description: "تشغيل FFmpeg Studio الآن"; Flags: nowait postinstall skipifsilent
