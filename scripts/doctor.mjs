import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

let failures = 0;
function report(name, ok, detail) {
  console.log(`${ok ? "OK" : "MISSING"} ${name}: ${detail}`);
  if (!ok) failures++;
}
function version(name, executable, args) {
  const result = spawnSync(executable, args, {
    encoding: "utf8",
    timeout: 10000,
  });
  const output = `${result.stdout || ""}${result.stderr || ""}`.trim();
  report(
    name,
    result.status === 0,
    result.status === 0 ? output.split("\n")[0] : "not available",
  );
  return result.status === 0 ? output : "";
}

report(
  "Node",
  Number(process.versions.node.split(".")[0]) >= 22,
  process.versions.node,
);
version("Rust", "rustc", ["--version"]);
version("Cargo", "cargo", ["--version"]);
version("Tauri CLI", "cargo", ["tauri", "--version"]);
version("JDK", "java", ["-version"]);
const targets = version("Rustup", "rustup", ["target", "list", "--installed"]);
for (const target of ["aarch64-linux-android", "x86_64-linux-android"]) {
  report(
    target,
    targets.split("\n").includes(target),
    targets.includes(target) ? "installed" : "rustup target add required",
  );
}

const sdk = process.env.ANDROID_HOME;
const ndk = process.env.NDK_HOME;
report(
  "Android SDK",
  !!sdk &&
    ["android-37", "android-37.0"].some((platform) =>
      existsSync(join(sdk, "platforms", platform, "android.jar")),
    ),
  "requires ANDROID_HOME and platform android-37",
);
report(
  "Build tools",
  !!sdk && existsSync(join(sdk, "build-tools/37.0.0")),
  "requires build-tools 37.0.0",
);
report(
  "Android NDK",
  !!ndk && existsSync(join(ndk, "source.properties")),
  "requires NDK_HOME",
);
if (ndk && existsSync(join(ndk, "source.properties"))) {
  const properties = readFileSync(join(ndk, "source.properties"), "utf8");
  report(
    "NDK version",
    properties.includes("29.0.14206865"),
    "expected 29.0.14206865",
  );
}
report(
  "Android host",
  existsSync("src-tauri/gen/android/app/build.gradle.kts"),
  "run Android initialization once, then npm run android:configure",
);
report(
  "Rust app lockfile",
  existsSync("src-tauri/Cargo.lock"),
  "resolve and commit on a network-enabled machine",
);
console.log(
  "Device access and authenticated Ubian behavior need separate validation.",
);
process.exitCode = failures ? 1 : 0;
