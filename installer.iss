; KatoS Interview Assistant — Inno Setup Script
; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
; НЕ ОТКРЫВАЙ ЭТОТ ФАЙЛ ВРУЧНУЮ В INNO SETUP.
; Запусти: python build_windows.py
; Он соберёт dist\ и сам скомпилирует установщик.
; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

#define AppName      "KatoS Interview Assistant"
#define AppVersion   "1.0"
#define AppPublisher "KatoS"
#define AppExeName   "KatoS_Interview_Assistant.exe"
#define SourceDir    "dist\KatoS_Interview_Assistant"

; Проверяем что dist\ существует — иначе ошибка с понятным текстом
#if !DirExists(SourceDir)
  #error "dist\KatoS_Interview_Assistant не найдена. Сначала запусти: python build_windows.py"
#endif

[Setup]
AppId={{A7F3B2C1-D4E5-4F6A-B7C8-D9E0F1A2B3C4}}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#AppExeName}
OutputDir=installer_output
OutputBaseFilename=KatoS_Interview_Assistant_Setup
Compression=lzma2/max
SolidCompression=yes
PrivilegesRequired=lowest

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Tasks]
Name: "desktopicon"; Description: "Create desktop shortcut"; GroupDescription: "Additional:"

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\{#AppName}";  Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill"; Parameters: "/F /IM {#AppExeName}"; Flags: runhidden skipifdoesntexist; RunOnceId: "KillProcess"

[UninstallDelete]
Type: filesandordirs; Name: "{app}"
Type: filesandordirs; Name: "{userappdata}\.katos_interview_assistant"
