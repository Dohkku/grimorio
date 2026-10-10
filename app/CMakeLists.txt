# Grimorio: la aplicación.
#
# Qt se enlaza SIEMPRE de forma dinámica: la licencia LGPLv3 lo exige para poder
# distribuir el binario sin abrir el código. Ver docs/DEPENDENCIAS.md.
cmake_minimum_required(VERSION 3.21)
project(grimorio VERSION 0.1.5 LANGUAGES CXX)

# La versión que se enseña (ajustes, «acerca de», propiedades del .exe). Por
# defecto la del `project()`; al publicar, la integración continua la saca de
# la etiqueta (`v0.2.0` → `-DGRIMORIO_VERSION=0.2.0`).
set(GRIMORIO_VERSION "${PROJECT_VERSION}" CACHE STRING "versión que se muestra")
if(NOT GRIMORIO_VERSION MATCHES "^([0-9]+)\\.([0-9]+)\\.([0-9]+)")
    message(FATAL_ERROR "GRIMORIO_VERSION tiene que empezar por X.Y.Z: «${GRIMORIO_VERSION}»")
endif()
set(VERSION_MAYOR ${CMAKE_MATCH_1})
set(VERSION_MENOR ${CMAKE_MATCH_2})
set(VERSION_PARCHE ${CMAKE_MATCH_3})

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

if(WIN32)
    # El icono y las propiedades del .exe van en un archivo de recursos. El C,
    # para el lanzador de la portable (más abajo).
    enable_language(RC)
    enable_language(C)
endif()
if(MSVC)
    # Rust enlaza su biblioteca estándar contra el runtime de C en DLL (/MD).
    # El programa tiene que usar el mismo, o el enlazador se queja de dos
    # runtimes. Es lo que CMake pone por defecto; se dice para no depender de
    # eso. (En Debug es /MDd, que con el puente compilado en /MD solo da un
    # aviso; para depurar en Windows mejor RelWithDebInfo.)
    set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL")
endif()

find_package(Qt6 6.2 REQUIRED COMPONENTS Core Gui Quick Qml OpenGL Network Test)
# D-Bus es para los portales del escritorio (diálogos y capturas), que solo
# existen en Linux: allí es obligatorio. En Windows no se busca.
if(UNIX AND NOT APPLE)
    find_package(Qt6 REQUIRED COMPONENTS DBus)
endif()
# En Windows los diálogos de archivos son QFileDialog, de QtWidgets (ver
# src/portal.cpp). En Linux no hace falta y no se enlaza.
if(WIN32)
    find_package(Qt6 REQUIRED COMPONENTS Widgets)
endif()

# --- el núcleo, compilado por cargo y enlazado estáticamente -----------------
# El puente es código propio, así que enlazarlo estático no plantea el problema
# de licencia que sí plantearía hacerlo con Qt.
set(RAIZ "${CMAKE_CURRENT_SOURCE_DIR}/..")

# Sin `--target`, cargo compila para la máquina en la que corre y deja el
# resultado en target/release. Con un triple (compilar para otra), lo deja en
# target/<triple>/release. Vacío es lo normal.
set(GRIMORIO_RUST_TARGET "" CACHE STRING "triple de Rust para el puente (vacío: el de esta máquina)")
set(CARGO_ARGS build --release -p grimorio-puente)
set(CARGO_SALIDA "${RAIZ}/target/release")
if(GRIMORIO_RUST_TARGET)
    list(APPEND CARGO_ARGS --target ${GRIMORIO_RUST_TARGET})
    set(CARGO_SALIDA "${RAIZ}/target/${GRIMORIO_RUST_TARGET}/release")
endif()
# Una biblioteca estática se llama `libX.a` con GCC y Clang, y `X.lib` con MSVC.
if(MSVC)
    set(PUENTE_A "${CARGO_SALIDA}/grimorio_puente.lib")
else()
    set(PUENTE_A "${CARGO_SALIDA}/libgrimorio_puente.a")
endif()

file(GLOB_RECURSE FUENTES_RUST
    "${CMAKE_CURRENT_SOURCE_DIR}/puente/src/*.rs"
    "${RAIZ}/crates/grimorio-core/src/*.rs")

add_custom_command(
    OUTPUT "${PUENTE_A}"
    COMMAND cargo ${CARGO_ARGS}
    WORKING_DIRECTORY "${RAIZ}"
    COMMENT "cargo build -p grimorio-puente"
    VERBATIM
    DEPENDS ${FUENTES_RUST} "${CMAKE_CURRENT_SOURCE_DIR}/puente/Cargo.toml"
)
add_custom_target(puente DEPENDS "${PUENTE_A}")

qt_add_executable(grimorio
    src/main.cpp
    src/nucleo.cpp
    src/modelo.cpp
    src/disposicion.cpp
    src/filas.cpp
    src/proveedor.cpp
    src/banco.cpp
    src/tema.cpp
    src/ajustes.cpp
    src/derivados.cpp
    src/fotogramas.cpp
    src/pixeles.cpp
    src/original.cpp
    src/portapapeles.cpp
    src/traer.cpp
    src/captura.cpp
    src/portal.cpp
    src/bibliotecas.cpp
    src/exportar.cpp
    src/vigilancia.cpp
    src/onda.cpp
    src/visor3d.cpp
    src/novedades.cpp
)
add_dependencies(grimorio puente)

qt_add_resources(grimorio "interfaz"
    PREFIX "/"
    FILES
        qml/Ventana.qml
        qml/BarraSuperior.qml
        qml/BarraLateral.qml
        qml/BarraEstado.qml
        qml/Bulto.qml
        qml/FilaCarpeta.qml
        qml/FilaNueva.qml
        qml/BotonIcono.qml
        qml/Icono.qml
        qml/Desplegable.qml
        qml/Chip.qml
        qml/AyudanteBusqueda.qml
        qml/VisorImagen.qml
        qml/MenuBiblioteca.qml
        qml/RenombrarLote.qml
        qml/GestorEtiquetas.qml
        qml/MenuCarpeta.qml
        qml/Malla.qml
        qml/Celda.qml
        qml/CabeceraLista.qml
        qml/Inspector.qml
        qml/Etiquetas.qml
        qml/Confirmar.qml
        qml/Estrellas.qml
        qml/Boton.qml
        qml/Divisor.qml
        qml/Visor.qml
        qml/Reproductor.qml
        qml/MallaVideos.qml
        qml/Transporte.qml
        qml/VisorSonido.qml
        qml/VisorPdf.qml
        qml/VisorModelo.qml
        qml/textos.js
        qml/consulta.js
        qml/iconos.js
        qml/PanelAjustes.qml
        qml/PanelVinculos.qml
        qml/PanelNovedades.qml
        qml/Pista.qml
        qml/EjesVista.qml
)

# Los temas que vienen dentro, para elegirlos en los ajustes sin saber dónde
# está el archivo. `--tema` sigue pudiendo cargar cualquier otro.
# Lo que trae cada versión, para enseñarlo al actualizar (novedades.h). El
# mismo archivo llena la release de GitHub (release.yml).
qt_add_resources(grimorio "novedades"
    PREFIX "/"
    BASE "${RAIZ}"
    FILES "${RAIZ}/NOVEDADES.md"
)

qt_add_resources(grimorio "temas"
    PREFIX "/temas"
    BASE "${RAIZ}/temas"
    FILES
        "${RAIZ}/temas/oscuro.json"
        "${RAIZ}/temas/papel.json"
)
# Solo para el C++: al compilador de recursos de Windows le llega por otro
# lado (ver más abajo).
target_compile_definitions(grimorio PRIVATE
    "$<$<COMPILE_LANGUAGE:CXX>:GRIMORIO_VERSION=\"${GRIMORIO_VERSION}\">")

# Lo que se publica (release.yml) sabe actualizarse solo desde el botón de
# Ajustes; un build de desarrollo no, para no pisarse con lo publicado.
option(GRIMORIO_PAQUETE "build de los paquetes publicados: se actualiza desde dentro" OFF)
if(GRIMORIO_PAQUETE)
    target_compile_definitions(grimorio PRIVATE GRIMORIO_PAQUETE)
endif()

# El icono de la ventana. En PNG y no en SVG: el SVG pediría el módulo QtSvg,
# y para un icono bastan los tamaños hechos (app/icono/LEEME.md).
qt_add_resources(grimorio "icono"
    PREFIX "/icono"
    BASE "${CMAKE_CURRENT_SOURCE_DIR}/icono"
    FILES
        icono/grimorio-16.png
        icono/grimorio-24.png
        icono/grimorio-32.png
        icono/grimorio-48.png
        icono/grimorio-64.png
        icono/grimorio-128.png
        icono/grimorio-256.png
)

target_include_directories(grimorio PRIVATE src)
target_link_libraries(grimorio PRIVATE
    Qt6::Core Qt6::Gui Qt6::Quick Qt6::Qml Qt6::OpenGL Qt6::Network
    "${PUENTE_A}"
)
if(TARGET Qt6::DBus)
    target_link_libraries(grimorio PRIVATE Qt6::DBus)
    target_compile_definitions(grimorio PRIVATE GRIMORIO_DBUS)
endif()

if(WIN32)
    target_link_libraries(grimorio PRIVATE
        Qt6::Widgets
        # Lo que arrastra la biblioteca estándar de Rust con MSVC. La lista es
        # la que da `cargo rustc -p grimorio-puente --release -- --print
        # native-static-libs`; bcrypt y advapi32 se añaden porque versiones
        # anteriores de Rust las pedían, y nombrarlas de más no cuesta nada.
        kernel32 ntdll userenv ws2_32 dbghelp shell32 legacy_stdio_definitions
        bcrypt advapi32
    )
    # Sin consola: es una aplicación de ventana. Lo que escribe por la salida
    # de error se sigue viendo si se lanza desde una terminal redirigiendo
    # (`grimorio.exe --bench 10 2>&1 | more`).
    set_target_properties(grimorio PROPERTIES WIN32_EXECUTABLE TRUE)
    # El icono del .exe (el del Explorador y la barra de tareas antes de que la
    # ventana ponga el suyo) y la ficha de versión de «Propiedades».
    target_sources(grimorio PRIVATE windows/grimorio.rc)
    # La versión le llega al .rc en una cabecera escrita aquí y no con `-D`:
    # una cadena entre comillas en la línea de órdenes del compilador de
    # recursos es justo lo que cada herramienta escapa a su manera.
    file(CONFIGURE OUTPUT "${CMAKE_CURRENT_BINARY_DIR}/version_rc.h" CONTENT
"#define VERSION_MAYOR @VERSION_MAYOR@
#define VERSION_MENOR @VERSION_MENOR@
#define VERSION_PARCHE @VERSION_PARCHE@
#define VERSION_TEXTO \"@GRIMORIO_VERSION@\"
")
    # Para que el compilador de recursos encuentre la cabecera, y el icono
    # tanto si resuelve las rutas desde el .rc como desde las carpetas de
    # inclusión. Para el C++ no cambia nada: ahí no hay nada que incluya.
    target_include_directories(grimorio PRIVATE
        "${CMAKE_CURRENT_BINARY_DIR}" "${CMAKE_CURRENT_SOURCE_DIR}/windows")

    # El `Grimorio.exe` de la portable, que arranca `programa\grimorio.exe`
    # (el porqué, en lanzador.c). Aquí se llama `lanzador.exe`: en una carpeta
    # de Windows, `Grimorio.exe` y `grimorio.exe` son el mismo archivo. Se
    # renombra al empaquetar (release.yml).
    add_executable(lanzador WIN32
        "${RAIZ}/installer/windows/portable/lanzador.c"
        "${RAIZ}/installer/windows/portable/lanzador.rc")
    target_compile_definitions(lanzador PRIVATE UNICODE _UNICODE)
    target_include_directories(lanzador PRIVATE
        "${CMAKE_CURRENT_BINARY_DIR}" "${CMAKE_CURRENT_SOURCE_DIR}/icono")
    target_link_libraries(lanzador PRIVATE user32)
else()
    target_link_libraries(grimorio PRIVATE
        # Lo que arrastran la biblioteca estándar de Rust y el SQLite empotrado.
        pthread dl m
    )
endif()

# --- pruebas -----------------------------------------------------------------
enable_testing()

# La geometría de la malla, probada sin ventana ni modelo.
qt_add_executable(prueba_filas pruebas/filas_test.cpp src/filas.cpp)
target_include_directories(prueba_filas PRIVATE src)
target_link_libraries(prueba_filas PRIVATE Qt6::Core Qt6::Test)
add_test(NAME filas COMMAND prueba_filas)

# Lo que decide si sale el aviso de versión nueva, sin red.
qt_add_executable(prueba_version pruebas/version_test.cpp)
target_include_directories(prueba_version PRIVATE src)
target_link_libraries(prueba_version PRIVATE Qt6::Core Qt6::Test)
add_test(NAME version COMMAND prueba_version)

# El arrastre del árbol, con las filas de verdad y el ratón moviéndose. Es lo
# único que caza que la carga llegue al otro lado y que las bandas caigan donde
# apunta el cursor: las dos cosas se veían bien en una captura y no funcionaban.
qt_add_executable(prueba_arrastre pruebas/arrastre_test.cpp src/tema.cpp)
target_include_directories(prueba_arrastre PRIVATE src)
target_compile_definitions(prueba_arrastre PRIVATE
    RUTA_ESCENA="${CMAKE_CURRENT_SOURCE_DIR}/pruebas/arrastre.qml")
target_link_libraries(prueba_arrastre PRIVATE Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick Qt6::Test)
# Sin pantalla: la prueba mueve el ratón por su cuenta y no mira nada dibujado.
add_test(NAME arrastre COMMAND prueba_arrastre -platform offscreen)

# El estilo es dato: ni un color ni una medida escritos a mano en el QML.
add_test(NAME sin_colores_a_mano
    COMMAND "${CMAKE_COMMAND}"
        -DQML_DIR=${CMAKE_CURRENT_SOURCE_DIR}/qml
        -P "${CMAKE_CURRENT_SOURCE_DIR}/pruebas/sin_colores_a_mano.cmake")

# qmllint caza lo que en QML solo se ve al ejecutar: propiedades que no existen,
# señales mal escritas, imports que faltan.
find_program(QMLLINT NAMES qmllint qmllint6 HINTS /usr/lib/qt6/bin)
if(QMLLINT)
    file(GLOB QMLS "${CMAKE_CURRENT_SOURCE_DIR}/qml/*.qml")
    # `Reproductor.qml` se mira aparte porque importa QtMultimedia, que es
    # opcional. Metido en el mismo montón, un paquete que falta haría saltar la
    # prueba entera y dejaría los otros catorce archivos sin revisar: un aviso
    # que se apaga solo es peor que no tenerlo.
    set(CON_MULTIMEDIA "${CMAKE_CURRENT_SOURCE_DIR}/qml/Reproductor.qml"
                       "${CMAKE_CURRENT_SOURCE_DIR}/qml/MallaVideos.qml")
    list(REMOVE_ITEM QMLS ${CON_MULTIMEDIA})
    # Los que usan `Onda` y `Visor3D`, que se registran desde C++ al arrancar y
    # qmllint no ve. Con ellos dentro, «Failed to import Grimorio» saltaba la
    # prueba entera y dejaba sin revisar todo lo demás.
    set(CON_TIPOS_PROPIOS "${CMAKE_CURRENT_SOURCE_DIR}/qml/VisorSonido.qml"
                          "${CMAKE_CURRENT_SOURCE_DIR}/qml/VisorModelo.qml")
    list(REMOVE_ITEM QMLS ${CON_TIPOS_PROPIOS})

    add_test(NAME qml_sin_avisos COMMAND "${QMLLINT}" ${QMLS})
    # Sin los paquetes `qml6-module-*` instalados, qmllint no resuelve ni un
    # tipo y escupe avisos en cascada que no dicen nada del código. En ese caso
    # la prueba se salta en vez de mentir diciendo que hay errores.
    set_tests_properties(qml_sin_avisos PROPERTIES
        SKIP_REGULAR_EXPRESSION "Failed to import")

    # `--type info` solo aquí: el QtMultimedia de Qt 6.4 declara `position` y
    # `duration` como `qlonglong`, un tipo que qmllint no tiene registrado, y
    # avisa cinco veces por algo que no está en nuestro código. Los otros
    # archivos siguen con la vara alta.
    add_test(NAME qml_reproductor COMMAND "${QMLLINT}" --type info ${CON_MULTIMEDIA})
    set_tests_properties(qml_reproductor PROPERTIES
        SKIP_REGULAR_EXPRESSION "Failed to import")

    # Sin el módulo propio, qmllint no sabe nada de `Onda` ni de `Visor3D`:
    # avisa de ellos como desconocidos. Se revisa todo lo demás de esos dos
    # archivos y se dejan pasar solo los avisos de importación y de tipo.
    add_test(NAME qml_tipos_propios COMMAND "${QMLLINT}" --import info --unqualified info --type info --property info
             ${CON_TIPOS_PROPIOS})
endif()
