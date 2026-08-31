import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  checkLinuxUpdate,
  compareSemver,
  defaultInstall,
  detectPackageType,
  githubAuthHeaders,
  pickAppImageAsset,
  pickDebAsset,
  pickRpmAsset,
  sha256ForName,
} from "./update.mjs";

test("compareSemver ordena tags con o sin v", () => {
  assert.equal(compareSemver("0.9.1", "0.9.0"), 1);
  assert.equal(compareSemver("v0.9.0", "0.9.0"), 0);
  assert.equal(compareSemver("0.8.9", "0.9.0"), -1);
});

test("pickDebAsset elige amd64 en x64", () => {
  const assets = [
    { name: "SHA256SUMS.txt" },
    { name: "LoLTerm-0.9.1-linux-amd64.deb" },
    { name: "LoLTerm-0.9.1-linux-arm64.deb" },
  ];
  assert.equal(pickDebAsset(assets, "x64").name, "LoLTerm-0.9.1-linux-amd64.deb");
  assert.equal(pickDebAsset(assets, "arm64").name, "LoLTerm-0.9.1-linux-arm64.deb");
});

test("pickRpmAsset elige x86_64 en x64", () => {
  const assets = [
    { name: "SHA256SUMS.txt" },
    { name: "LoLTerm-0.9.1-linux-x86_64.rpm" },
    { name: "LoLTerm-0.9.1-linux-aarch64.rpm" },
  ];
  assert.equal(pickRpmAsset(assets, "x64").name, "LoLTerm-0.9.1-linux-x86_64.rpm");
  assert.equal(pickRpmAsset(assets, "arm64").name, "LoLTerm-0.9.1-linux-aarch64.rpm");
});

test("pickAppImageAsset elige x86_64 en x64", () => {
  const assets = [
    { name: "SHA256SUMS.txt" },
    { name: "LoLTerm-0.9.1-linux-x86_64.AppImage" },
    { name: "LoLTerm-0.9.1-linux-aarch64.AppImage" },
  ];
  assert.equal(pickAppImageAsset(assets, "x64").name, "LoLTerm-0.9.1-linux-x86_64.AppImage");
  assert.equal(pickAppImageAsset(assets, "arm64").name, "LoLTerm-0.9.1-linux-aarch64.AppImage");
});

test("detectPackageType respeta el override explícito", () => {
  assert.equal(detectPackageType({ packageType: "rpm" }), "rpm");
  assert.equal(detectPackageType({ packageType: "deb" }), "deb");
  assert.equal(detectPackageType({ packageType: "appimage" }), "appimage");
});

test("detectPackageType prefiere APPIMAGE sobre os-release", () => {
  const prev = process.env.APPIMAGE;
  process.env.APPIMAGE = "/tmp/LoLTerm.AppImage";
  try {
    assert.equal(detectPackageType({ osReleaseText: "ID=fedora\n" }), "appimage");
  } finally {
    if (prev === undefined) delete process.env.APPIMAGE;
    else process.env.APPIMAGE = prev;
  }
});

test("defaultInstall appimage reemplaza el AppImage en ejecución", async () => {
  const dir = mkdtempSync(join(tmpdir(), "lolterm-upd-"));
  const target = join(dir, "LoLTerm-0.13.4.AppImage");
  const pkg = join(dir, "LoLTerm-0.14.0.AppImage");
  writeFileSync(target, "old");
  writeFileSync(pkg, "new");
  const prev = process.env.APPIMAGE;
  process.env.APPIMAGE = target;
  try {
    const method = await defaultInstall(pkg, "appimage");
    assert.equal(method, "appimage");
    assert.equal(readFileSync(target, "utf8"), "new");
    assert.notEqual(statSync(target).mode & 0o111, 0);
  } finally {
    if (prev === undefined) delete process.env.APPIMAGE;
    else process.env.APPIMAGE = prev;
    rmSync(dir, { recursive: true, force: true });
  }
});

test("detectPackageType clasifica os-release de distros rpm y deb", () => {
  assert.equal(detectPackageType({ osReleaseText: 'ID=fedora\nID_LIKE="fedora"\n' }), "rpm");
  assert.equal(detectPackageType({ osReleaseText: 'ID="rocky"\nID_LIKE="rhel centos fedora"\n' }), "rpm");
  assert.equal(detectPackageType({ osReleaseText: "ID=ubuntu\nID_LIKE=debian\n" }), "deb");
  assert.equal(detectPackageType({ osReleaseText: "ID=debian\n" }), "deb");
});

test("sha256ForName lee el formato de sha256sum", () => {
  const sums = [
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  LoLTerm-0.9.1-linux-amd64.deb",
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb *otro.deb",
  ].join("\n");
  assert.equal(
    sha256ForName(sums, "/tmp/LoLTerm-0.9.1-linux-amd64.deb"),
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  );
});

test("githubAuthHeaders no manda Authorization vacío", () => {
  assert.deepEqual(githubAuthHeaders(""), {});
  assert.equal(githubAuthHeaders("fixture-token").Authorization, "Bearer fixture-token");
});

test("checkLinuxUpdate trata 404 como sin update", async () => {
  const info = await checkLinuxUpdate({
    currentVersion: "0.9.0",
    allowNonLinux: true,
    packageType: "deb",
    token: " ",
    fetchFn: async () => new Response("Not Found", { status: 404 }),
  });
  assert.equal(info.available, false);
  assert.equal(info.reason, "github-404");
});

test("checkLinuxUpdate elige el rpm en modo rpm", async () => {
  const release = {
    tag_name: "v0.9.1",
    body: "",
    assets: [
      { name: "SHA256SUMS.txt", browser_download_url: "https://github.com/LautaroPiugh/lolterm/releases/download/v0.9.1/SHA256SUMS.txt" },
      { name: "LoLTerm-0.9.1-linux-amd64.deb", browser_download_url: "https://github.com/example.deb" },
      { name: "LoLTerm-0.9.1-linux-x86_64.rpm", browser_download_url: "https://github.com/example.rpm" },
    ],
  };
  const info = await checkLinuxUpdate({
    currentVersion: "0.9.0",
    allowNonLinux: true,
    packageType: "rpm",
    fetchFn: async () => new Response(JSON.stringify(release), { status: 200 }),
  });
  assert.equal(info.available, true);
  assert.equal(info.packageType, "rpm");
  assert.equal(info.assetName, "LoLTerm-0.9.1-linux-x86_64.rpm");
});

test("checkLinuxUpdate elige el AppImage en modo portable", async () => {
  const release = {
    tag_name: "v0.9.1",
    body: "",
    assets: [
      { name: "SHA256SUMS.txt", browser_download_url: "https://github.com/LautaroPiugh/lolterm/releases/download/v0.9.1/SHA256SUMS.txt" },
      { name: "LoLTerm-0.9.1-linux-amd64.deb", browser_download_url: "https://github.com/example.deb" },
      { name: "LoLTerm-0.9.1-linux-x86_64.AppImage", browser_download_url: "https://github.com/example.appimage" },
    ],
  };
  const info = await checkLinuxUpdate({
    currentVersion: "0.9.0",
    allowNonLinux: true,
    packageType: "appimage",
    fetchFn: async () => new Response(JSON.stringify(release), { status: 200 }),
  });
  assert.equal(info.available, true);
  assert.equal(info.packageType, "appimage");
  assert.equal(info.assetName, "LoLTerm-0.9.1-linux-x86_64.AppImage");
});
