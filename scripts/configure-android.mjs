import {
  existsSync,
  readFileSync,
  writeFileSync,
  mkdirSync,
  copyFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

// Kept outside the generated host. Reapply after Android init/regeneration.
export function configureAndroid(root) {
  const app = join(root, "src-tauri/gen/android/app");
  const manifestFile = join(app, "src/main/AndroidManifest.xml");
  const gradleFile = join(app, "build.gradle.kts");
  if (!existsSync(manifestFile) || !existsSync(gradleFile)) {
    throw new Error(
      "Android host is missing. Run npm run tauri -- android init once.",
    );
  }
  let gradle = readFileSync(gradleFile, "utf8");
  for (const [key, value] of [
    ["compileSdk", "37"],
    ["minSdk", "26"],
    ["targetSdk", "37"],
  ]) {
    const pattern = new RegExp(`\\b${key}\\s*=\\s*\\d+`);
    if (!pattern.test(gradle))
      throw new Error(`Unknown Android template: missing ${key}`);
    gradle = gradle.replace(pattern, `${key} = ${value}`);
  }
  for (const [key, value] of [
    ["ndkVersion", "29.0.14206865"],
    ["buildToolsVersion", "37.0.0"],
  ]) {
    const anyAssignment = new RegExp(`\\b${key}\\s*=`);
    const literalAssignment = new RegExp(`\\b${key}\\s*=\\s*"[^"]+"`);
    if (anyAssignment.test(gradle)) {
      if (!literalAssignment.test(gradle))
        throw new Error(`Unknown Android template: nonliteral ${key}`);
      gradle = gradle.replace(literalAssignment, `${key} = "${value}"`);
    } else {
      if (!/android\s*\{/.test(gradle))
        throw new Error("Unknown Android template: missing android block");
      gradle = gradle.replace(
        /android\s*\{/,
        `android {\n    ${key} = "${value}"`,
      );
    }
  }

  let manifest = readFileSync(manifestFile, "utf8");
  if (!manifest.includes("xmlns:tools=")) {
    manifest = manifest.replace(
      "<manifest ",
      '<manifest xmlns:tools="http://schemas.android.com/tools" ',
    );
  }
  if (!/<application\b[^>]*>/.test(manifest))
    throw new Error("Unknown Android manifest template");
  manifest = manifest.replace(/<application\b[^>]*>/, (tag) => {
    const attributes = {
      "android:allowBackup": "false",
      "android:fullBackupContent": "@xml/backup_rules",
      "android:dataExtractionRules": "@xml/data_extraction_rules",
      "android:usesCleartextTraffic": "false",
      "android:networkSecurityConfig": "@xml/network_security_config",
    };
    for (const [key, value] of Object.entries(attributes)) {
      const pattern = new RegExp(`\\s${key}="[^"]*"`);
      tag = pattern.test(tag)
        ? tag.replace(pattern, ` ${key}="${value}"`)
        : tag.replace(/>$/, `\n        ${key}="${value}">`);
    }
    const old = tag.match(/tools:replace="([^"]*)"/)?.[1]?.split(",") || [];
    const replace = [...new Set([...old, ...Object.keys(attributes)])].join(
      ",",
    );
    tag = tag.replace(/\s*tools:replace="[^"]*"/, "");
    return tag.replace(/>$/, `\n        tools:replace="${replace}">`);
  });

  // Fail before any writes if the generated template is unfamiliar.
  writeFileSync(gradleFile, gradle);
  writeFileSync(manifestFile, manifest);
  const native = join(root, "plugins/android-platform/android/src");
  for (const [source, destination] of [
    ["main/res/xml/backup_rules.xml", "main/res/xml/backup_rules.xml"],
    [
      "main/res/xml/data_extraction_rules.xml",
      "main/res/xml/data_extraction_rules.xml",
    ],
    [
      "main/res/xml/network_security_config.xml",
      "main/res/xml/network_security_config.xml",
    ],
    [
      "debug/res/xml/network_security_config.xml",
      "debug/res/xml/network_security_config.xml",
    ],
  ]) {
    const path = join(app, "src", destination);
    mkdirSync(dirname(path), { recursive: true });
    copyFileSync(join(native, source), path);
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  try {
    configureAndroid(process.cwd());
    console.log("Applied Android SDK, backup and HTTPS configuration.");
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
