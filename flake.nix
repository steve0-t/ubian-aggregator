{
  description = "ubian-aggregator — Rust, Svelte, Tauri and Android development environment";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-26.05";
    nixpkgs-unstable.url = "nixpkgs/nixos-unstable";
  };

  outputs = {
    nixpkgs,
    nixpkgs-unstable,
    ...
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      config = {
        allowUnfree = true;
        android_sdk.accept_license = true;
      };
    };
    pkgs-unstable = nixpkgs-unstable.legacyPackages.${system};
    android = pkgs.androidenv.composeAndroidPackages {
      cmdLineToolsVersion = "20.0";
      toolsVersion = null;
      # Tauri's Android library compiles against 36; the app targets 37.
      platformVersions = ["36" "37.0"];
      buildToolsVersions = ["36.0.0" "37.0.0"];
      includeNDK = true;
      ndkVersions = ["29.0.14206865"];
      includeCmake = false;
      includeEmulator = false;
    };
    androidSdk = "${android.androidsdk}/libexec/android-sdk";
  in {
    devShells.${system}.default = pkgs.mkShell {
      packages = with pkgs; [
        # rustup supplies Cargo/rustc shims so installed Android targets are used.
        rustup
        nodejs_22
        pkg-config
        jdk17
        android-tools
        android.androidsdk
      ];
      buildInputs = with pkgs; [
        webkitgtk_4_1
        gtk3
        libsoup_3
        openssl
        librsvg
      ];
      shellHook = ''
        export JAVA_HOME="${pkgs.jdk17}"
        export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
        export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}"
        # Reuse a caller-provided SDK; replace stale/empty shell defaults.
        if [ ! -d "''${ANDROID_HOME:-}/platforms" ]; then
          if [ -d "''${ANDROID_SDK_ROOT:-}/platforms" ]; then
            export ANDROID_HOME="$ANDROID_SDK_ROOT"
          else
            export ANDROID_HOME="${androidSdk}"
          fi
        fi
        export ANDROID_SDK_ROOT="$ANDROID_HOME"
        if [ ! -f "''${NDK_HOME:-}/source.properties" ]; then
          export NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"
        fi
        export PATH="$HOME/.cargo/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$PATH"
        # Use the SDK's patched binary on NixOS instead of Maven's aapt2.
        export GRADLE_OPTS="''${GRADLE_OPTS:-} -Dorg.gradle.project.android.aapt2FromMavenOverride=$ANDROID_HOME/build-tools/37.0.0/aapt2"
      '';
    };
  };
}
