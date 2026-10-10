; Instalador de Grimorio para Windows (Inno Setup 6).
;
; No compila nada: empaqueta una carpeta ya preparada, la que deja el flujo de
; publicación (.github/workflows/release.yml) con grimorio.exe, grim.exe, las
; DLL de Qt que pone windeployqt, el runtime de Visual C++ y los avisos de
; licencias. Para hacerlo a mano:
;
;   iscc /DVersion=0.1.0 /DOrigen=C:\ruta\a\dist\Grimorio /DSalida=C:\ruta\a\dist installer\windows\grimorio.iss
;
; Se puede instalar para todos (pide permisos de administrador, va a
; Archivos de programa) o solo para quien lo instala (sin permisos, va a
; %LOCALAPPDATA%\Programs). Lo pregunta el propio instalador al empezar: en
; un equipo del trabajo sin permisos de administrador también tiene que poder
; instalarse.
;
; No se asocia ninguna extensión: una biblioteca de Grimorio es una carpeta,
; y no hay archivo al que hacer doble clic.

#ifndef Version
  #define Version "0.0.0"
#endif
#ifndef VersionNumerica
  #define VersionNumerica Version
#endif
; Las rutas relativas son desde la carpeta de este archivo.
#ifndef Origen
  #define Origen "..\..\dist\Grimorio"
#endif
#ifndef Salida
  #define Salida "..\..\dist"
#endif

[Setup]
; El identificador no se cambia nunca: es como Windows reconoce que una
; versión nueva actualiza a la anterior en vez de instalarse al lado.
AppId={{37D19246-7DDB-4198-BF37-54366DDE2E6B}
AppName=Grimorio
AppVersion={#Version}
AppVerName=Grimorio {#Version}
AppPublisher=Frederick Andrade
AppPublisherURL=https://grimorio.frederickandrade.com
AppSupportURL=https://grimorio.frederickandrade.com
AppUpdatesURL=https://grimorio.frederickandrade.com
; Solo números (X.Y.Z): «0.2.0-beta» no vale como versión de archivo.
VersionInfoVersion={#VersionNumerica}
DefaultDirName={autopf}\Grimorio
DefaultGroupName=Grimorio
DisableProgramGroupPage=yes
; Para todos por defecto, con la opción de instalar solo para uno.
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog
; Solo 64 bits: así se compila Qt y así se compila el núcleo.
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Qt 6 pide Windows 10 o posterior.
MinVersion=10.0
LicenseFile=..\..\LICENSE.md
SetupIconFile=..\..\app\icono\grimorio.ico
UninstallDisplayIcon={app}\grimorio.exe
UninstallDisplayName=Grimorio
OutputDir={#Salida}
OutputBaseFilename=Grimorio-{#Version}-windows-x64-instalador
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; Si Grimorio está abierto al actualizar, se ofrece cerrarlo: con el .exe en
; uso no se puede reemplazar.
CloseApplications=yes

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "escritorio"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#Origen}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Grimorio"; Filename: "{app}\grimorio.exe"
Name: "{autodesktop}\Grimorio"; Filename: "{app}\grimorio.exe"; Tasks: escritorio

[Run]
Filename: "{app}\grimorio.exe"; Description: "{cm:LaunchProgram,Grimorio}"; Flags: nowait postinstall skipifsilent
; Lanzado por el botón de actualizar de Grimorio (`/SILENT /actualizar=1`):
; sin ventanas, y al acabar se vuelve a abrir. Como quien lo usa, no como
; administrador, aunque la instalación lo haya pedido.
Filename: "{app}\grimorio.exe"; Flags: nowait runasoriginaluser; Check: DesdeGrimorio

; Al desinstalar se borra solo lo instalado. Las bibliotecas (por defecto
; Documentos\Grimorio.grimorio) y los ajustes (%APPDATA%\Grimorio) son de la
; persona y se quedan.

; La sección de código va la última: Inno Setup lee como Pascal todo lo que
; viene detrás, comentarios con «;» incluidos.
[Code]
function DesdeGrimorio: Boolean;
begin
  Result := ExpandConstant('{param:actualizar|0}') = '1';
end;
