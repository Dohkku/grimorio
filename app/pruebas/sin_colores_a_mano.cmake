# Centinela del estilo-como-dato, gemelo del que vigila el spike de wgpu.
#
# Si alguien escribe un color o una medida a mano en el QML, el tema deja de ser
# la única fuente de verdad y la promesa de "los visuales se pueden cambiar" se
# rompe en silencio. Esta prueba hace ruido.

file(GLOB_RECURSE QMLS "${QML_DIR}/*.qml")
if(QMLS STREQUAL "")
    message(FATAL_ERROR "no encontré ningún .qml en ${QML_DIR}")
endif()

set(CULPAS "")
foreach(F ${QMLS})
    file(STRINGS "${F}" LINEAS)
    set(N 0)
    foreach(L ${LINEAS})
        math(EXPR N "${N}+1")
        # Los comentarios explican, no pintan.
        if(L MATCHES "^[ \t]*//")
            continue()
        endif()
        if(L MATCHES "#[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]")
            list(APPEND CULPAS "${F}:${N}: color escrito a mano → ${L}")
        endif()
        # `Qt.rgba(...)` y `"red"` son la misma trampa por otra puerta.
        if(L MATCHES "Qt\\.(rgba|hsla|hsva|lighter|darker)\\(")
            list(APPEND CULPAS "${F}:${N}: color construido a mano → ${L}")
        endif()
        if(L MATCHES "color:[ \t]*\"[a-z]" AND NOT L MATCHES "\"transparent\"")
            list(APPEND CULPAS "${F}:${N}: color con nombre → ${L}")
        endif()
    endforeach()
endforeach()

if(NOT CULPAS STREQUAL "")
    string(REPLACE ";" "\n  " TEXTO "${CULPAS}")
    message(FATAL_ERROR
        "el estilo tiene que salir del tema, no del código:\n  ${TEXTO}")
endif()
message(STATUS "sin colores escritos a mano en el QML")
