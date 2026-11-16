import test from "node:test";
import assert from "node:assert/strict";
import {
  readFileSync,
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { configureAndroid } from "../scripts/configure-android.mjs";

const read = (path) =>
  readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

test("privileged main view has no remote capability or website frame access", () => {
  const config = JSON.parse(read("src-tauri/tauri.conf.json"));
  const capability = JSON.parse(read("src-tauri/capabilities/local-main.json"));
  assert.equal(capability.local, true);
  assert.equal(capability.remote, undefined);
  assert.deepEqual(capability.windows, ["main"]);
  assert.deepEqual(capability.permissions.sort(), [
    "android-platform:allow-keystore-probe",
    "android-platform:allow-open-provider",
    "android-platform:allow-status",
  ]);
  assert.match(config.app.security.csp, /frame-src 'none'/);
  assert.match(config.app.security.csp, /script-src 'self';/);
  assert.match(config.app.security.csp, /form-action 'none'/);
  assert.equal(config.build.frontendDist, "../dist");
  assert.equal(config.app.windows[0].create, false);
});

test("shipping policy does not invent observed analytics vendors", () => {
  const policy = JSON.parse(read("policy/network-policy.json"));
  assert.equal(policy.version, 1);
  assert.deepEqual(policy.providerHosts, ["www.ubian.sk"]);
  assert.deepEqual(policy.optionalRules, []);
});

test("generated Android host keeps backup exclusions and HTTPS after regeneration", () => {
  const root = mkdtempSync(join(tmpdir(), "cestou-host-"));
  const app = join(root, "src-tauri/gen/android/app");
  try {
    mkdirSync(join(app, "src/main"), { recursive: true });
    writeFileSync(
      join(app, "src/main/AndroidManifest.xml"),
      '<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application android:allowBackup="true" android:usesCleartextTraffic="${usesCleartextTraffic}"></application></manifest>',
    );
    writeFileSync(
      join(app, "build.gradle.kts"),
      "android {\n compileSdk = 35\n defaultConfig { minSdk = 24\n targetSdk = 35 }\n}\n",
    );
    for (const relative of [
      "main/res/xml/backup_rules.xml",
      "main/res/xml/data_extraction_rules.xml",
      "main/res/xml/network_security_config.xml",
      "debug/res/xml/network_security_config.xml",
    ]) {
      const destination = join(
        root,
        "plugins/android-platform/android/src",
        relative,
      );
      mkdirSync(dirname(destination), { recursive: true });
      writeFileSync(
        destination,
        read(`plugins/android-platform/android/src/${relative}`),
      );
    }
    configureAndroid(root);
    const first = readFileSync(
      join(app, "src/main/AndroidManifest.xml"),
      "utf8",
    );
    assert.match(first, /android:allowBackup="false"/);
    assert.match(first, /android:usesCleartextTraffic="false"/);
    assert.match(
      first,
      /android:dataExtractionRules="@xml\/data_extraction_rules"/,
    );
    const rules = readFileSync(
      join(app, "src/main/res/xml/data_extraction_rules.xml"),
      "utf8",
    );
    for (const transport of ["cloud-backup", "device-transfer"]) {
      const section = rules.match(
        new RegExp(`<${transport}[^>]*>([\\s\\S]*?)</${transport}>`),
      )[1];
      assert.match(section, /domain="root" path="\."/);
      assert.match(section, /domain="device_root" path="\."/);
    }
    const network = readFileSync(
      join(app, "src/main/res/xml/network_security_config.xml"),
      "utf8",
    );
    assert.doesNotMatch(network, /cleartextTrafficPermitted="true"/);
    configureAndroid(root);
    assert.equal(
      readFileSync(join(app, "src/main/AndroidManifest.xml"), "utf8"),
      first,
    );
    writeFileSync(join(app, "build.gradle.kts"), "android { }");
    assert.throws(() => configureAndroid(root), /Unknown Android template/);
    assert.equal(
      readFileSync(join(app, "src/main/AndroidManifest.xml"), "utf8"),
      first,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
