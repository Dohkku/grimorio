// Un `chrome.i18n` de mentira con los textos de _locales/, para probar el
// código fuera del navegador. Por defecto en español, como lo vería un
// navegador en español: así las pruebas comparan con el texto de siempre.
import { readFileSync } from "node:fs";

export function mensajes(idioma) {
  return JSON.parse(readFileSync(new URL(`../_locales/${idioma}/messages.json`, import.meta.url), "utf8"));
}

/// Lo mismo que hace el navegador: los `$nombre$` del mensaje se cambian por
/// el contenido de su placeholder, y los `$1`… por los huecos que se pasan.
export function getMessageDe(idioma) {
  const m = mensajes(idioma);
  return (clave, huecos = []) => {
    const e = m[clave];
    if (!e) return "";
    const lista = [].concat(huecos);
    const conHuecos = (s) => s.replace(/\$(\d)/g, (_, n) => lista[n - 1] ?? "");
    return e.message.replace(/\$([A-Za-z0-9_]+)\$/g, (_, p) => {
      const ph = e.placeholders && e.placeholders[p.toLowerCase()];
      return ph ? conHuecos(ph.content) : "";
    });
  };
}

export function ponerIdioma(idioma) {
  globalThis.chrome = { ...(globalThis.chrome || {}), i18n: { getMessage: getMessageDe(idioma) } };
}

ponerIdioma("es");
