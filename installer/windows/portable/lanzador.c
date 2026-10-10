// El `Grimorio.exe` de la versión portable: arranca `programa\grimorio.exe`.
//
// La portable es una carpeta descomprimida, y el programa de verdad lleva al
// lado unas doscientas cosas: las DLL de Qt, sus complementos, los módulos de
// QML, licencias. Abrir el zip y encontrarse eso asusta a quien no lo espera,
// y no se sabe cuál de todos hay que abrir. Windows solo busca las DLL junto
// al .exe, así que no se pueden mover sin más a una subcarpeta: van todas en
// `programa\` con el programa, y arriba queda este lanzador con el icono, un
// LEEME y nada más.
//
// Hace lo mínimo: se busca a sí mismo, arranca el de `programa\` con los
// mismos argumentos (para que soltar una carpeta encima siga funcionando) y se
// va. Sin consola, sin esperar, sin nada que mantener.
#define WIN32_LEAN_AND_MEAN
#ifndef UNICODE
#define UNICODE
#endif
#ifndef _UNICODE
#define _UNICODE
#endif
#include <windows.h>

#define TOPE 32768

static void avisar(const wchar_t *mensaje)
{
    MessageBoxW(NULL, mensaje, L"Grimorio", MB_OK | MB_ICONERROR);
}

int WINAPI wWinMain(HINSTANCE instancia, HINSTANCE previa, PWSTR argumentos, int mostrar)
{
    (void)instancia;
    (void)previa;
    (void)mostrar;

    static wchar_t programa[TOPE];
    DWORD n = GetModuleFileNameW(NULL, programa, TOPE);
    if (n == 0 || n >= TOPE) return 1;
    wchar_t *barra = NULL;
    for (wchar_t *c = programa; *c; ++c)
        if (*c == L'\\' || *c == L'/') barra = c;
    if (!barra) return 1;
    barra[1] = 0;

    static const wchar_t DENTRO[] = L"programa\\grimorio.exe";
    if (lstrlenW(programa) + lstrlenW(DENTRO) + 1 > TOPE) return 1;
    lstrcatW(programa, DENTRO);

    if (GetFileAttributesW(programa) == INVALID_FILE_ATTRIBUTES) {
        avisar(L"No encuentro programa\\grimorio.exe al lado de este Grimorio.exe.\n\n"
               L"Descomprime el zip entero y abre Grimorio.exe desde la carpeta descomprimida, "
               L"no desde dentro del zip.");
        return 1;
    }

    // La línea de órdenes: la ruta entre comillas y lo que nos pasaron, tal
    // cual llegó (wWinMain ya lo da sin el nombre del lanzador).
    static wchar_t linea[TOPE];
    const int largo = lstrlenW(programa) + lstrlenW(argumentos) + 4;
    if (largo > TOPE) return 1;
    lstrcpyW(linea, L"\"");
    lstrcatW(linea, programa);
    lstrcatW(linea, L"\"");
    if (argumentos && argumentos[0]) {
        lstrcatW(linea, L" ");
        lstrcatW(linea, argumentos);
    }

    STARTUPINFOW si;
    PROCESS_INFORMATION pi;
    ZeroMemory(&si, sizeof si);
    si.cb = sizeof si;
    ZeroMemory(&pi, sizeof pi);
    if (!CreateProcessW(programa, linea, NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) {
        avisar(L"No he podido abrir programa\\grimorio.exe.");
        return 1;
    }
    // Para que Windows pase al programa el permiso de ponerse delante.
    AllowSetForegroundWindow(pi.dwProcessId);
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
    return 0;
}
