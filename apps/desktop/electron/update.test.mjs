import assert from "node:assert/strict";
import { test } from "node:test";
import { checkLinuxDebUpdate, compareSemver, githubAuthHeaders, pickDebAsset, sha256ForName } from "./update.mjs";

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
  assert.equal(githubAuthHeaders("ghp_x").Authorization, "Bearer ghp_x");
});

test("checkLinuxDebUpdate trata 404 como sin update", async () => {
  const info = await checkLinuxDebUpdate({
    currentVersion: "0.9.0",
    allowNonLinux: true,
    token: " ",
    fetchFn: async () => new Response("Not Found", { status: 404 }),
  });
  assert.equal(info.available, false);
  assert.equal(info.reason, "github-404");
});
