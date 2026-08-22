// electron-builder llama esto tras stagear linux-unpacked y antes de empaquetar.
// Chromium exige chrome-sandbox con SUID y dueño root para poder sandboxear renderers.
const { chmodSync } = require("node:fs");
const path = require("node:path");

exports.default = async function afterPack(context) {
  if (context.electronPlatformName !== "linux") return;
  const helper = path.join(context.appOutDir, "chrome-sandbox");
  chmodSync(helper, 0o4755);
};
