# Seguridad

LoLTerm es local-first: no hay cuenta de LoLTerm, backend cloud obligatorio ni telemetría automática. Aun así, el producto ejecuta shells, SSH, herramientas CLI y una vista HTTP opcional, por lo que la frontera de seguridad importa.

## Reportar vulnerabilidades

No abras un issue público con tokens, rutas privadas, capturas con secretos ni payloads explotables. Usá GitHub Security Advisories si está habilitado en el repositorio; si no lo está, abrí un issue mínimo pidiendo un canal privado sin incluir detalles sensibles.

Incluí cuando sea seguro:

- versión de LoLTerm;
- sistema operativo y entorno de escritorio;
- pasos de reproducción;
- impacto esperado;
- si el problema requiere activar HTTP LAN, SSH, REST o agentes.

## Qué datos no debería guardar LoLTerm

LoLTerm no debe guardar en configuración portable:

- passwords;
- tokens;
- claves privadas SSH;
- valores completos de variables de entorno sensibles;
- credenciales de proveedores de IA.

Los archivos sincronizables viven en `~/.config/lolterm/`. Tratá esa carpeta como configuración portable, no como bóveda de secretos.

## HTTP LAN opt-in

La vista HTTP es opcional y está pensada para LAN/VPN o túnel SSH. No trae TLS propio.

Riesgo concreto: si activás HTTP en una red no confiable, otra persona de esa red podría intentar acceder al workspace. La password se guarda en el directorio de datos local de LoLTerm y debe tener al menos 8 caracteres.

Uso recomendado:

- mantenelo apagado salvo que lo necesites;
- usalo detrás de VPN/Tailscale o túnel SSH;
- no lo expongas directo a Internet;
- cambiá la password si compartiste la red o el equipo.

## Updates `.deb`

El updater de Linux consulta la última release estable de GitHub, descarga el `.deb` de esa misma release y verifica `SHA256SUMS.txt` antes de instalar.

Estado actual:

- origen permitido: GitHub por HTTPS;
- artefacto esperado: `LoLTerm-*-linux-*.deb`;
- integridad: SHA256;
- instalación: acción explícita del usuario con `pkexec apt-get install` o instalador del sistema;
- firma GPG/apt repo: todavía no.

SHA256 detecta corrupción o reemplazo accidental del artefacto, pero no reemplaza una firma criptográfica de release.

## Electron y preload

La UI corre en Electron. El renderer no debe tener APIs Node arbitrarias; el preload expone una superficie limitada hacia el proceso main. En desarrollo puede usarse `--no-sandbox` por limitaciones del binario local de Electron; el paquete `.deb` configura `chrome-sandbox` para usar el sandbox de Chromium.

## SSH y Tailscale

LoLTerm usa el `ssh` del sistema y las claves/agent existentes. No guarda credenciales SSH propias. Las contraseñas o passphrases las pide `ssh` dentro del PTY.

## Agentes y cuotas

LoLTerm puede mostrar estado/cuotas de CLIs instaladas. Para eso puede invocar herramientas locales o leer stores que esas CLIs ya crearon en el disco del usuario. LoLTerm no debe copiar ni persistir esos tokens como secretos propios.

Si no querés que una herramienta sea inspeccionada, no la abras desde LoLTerm o eliminá/deslogueá la CLI correspondiente.

## Diagnósticos

Los diagnósticos son locales y no se envían solos. Al abrir un issue, revisá el texto antes de publicarlo. LoLTerm intenta evitar salida de PTY y secretos, pero un mensaje de error podría contener contexto de tu máquina o proyecto.