import assert from "node:assert/strict";
import { test } from "node:test";
import {
  checkLinuxUpdate,
  compareSemver,
  detectPackageType,
  githubAuthHeaders,
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

test("detectPackageType respeta el override explícito", () => {
  assert.equal(detectPackageType({ packageType: "rpm" }), "rpm");
  assert.equal(detectPackageType({ packageType: "deb" }), "deb");
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
