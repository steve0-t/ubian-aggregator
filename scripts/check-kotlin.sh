#!/usr/bin/env bash
set -euo pipefail

# Optional host check when an Android SDK and Kotlin compiler are available.
# This excludes the Tauri-facing plugin class, which needs the generated host.
: "${ANDROID_JAR:?Set ANDROID_JAR to an installed Android SDK android.jar}"
: "${KOTLIN_COMPILER_CLASSPATH:?Set KOTLIN_COMPILER_CLASSPATH to the compiler and its runtime jars}"
: "${KOTLIN_STDLIB:?Set KOTLIN_STDLIB to the Kotlin standard-library jar}"
task_tmp=$(mktemp -d)
trap 'rm -rf "$task_tmp"' EXIT
node scripts/kotlin-url-harness.mjs "$task_tmp/PolicyHarness.kt"
task_java="${JAVA_HOME:+$JAVA_HOME/bin/}java"
"$task_java" -cp "$KOTLIN_COMPILER_CLASSPATH" org.jetbrains.kotlin.cli.jvm.K2JVMCompiler \
  -no-stdlib -no-reflect -jvm-target 1.8 \
  -classpath "$KOTLIN_COMPILER_CLASSPATH:$ANDROID_JAR" \
  -d "$task_tmp/platform.jar" \
  plugins/android-platform/android/src/main/java/sk/cestou/platform/RequestPolicy.kt \
  plugins/android-platform/android/src/main/java/sk/cestou/platform/ResourceFilter.kt \
  plugins/android-platform/android/src/main/java/sk/cestou/platform/ProviderSession.kt \
  plugins/android-platform/android/src/main/java/sk/cestou/platform/ProviderActivity.kt \
  plugins/android-platform/android/src/main/java/sk/cestou/platform/ProbeVault.kt \
  "$task_tmp/PolicyHarness.kt"
"$task_java" -cp "$task_tmp/platform.jar:$KOTLIN_STDLIB:$ANDROID_JAR" sk.cestou.platform.PolicyHarnessKt
