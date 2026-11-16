{
  description = "Cestou — Rust, Svelte, Tauri and Android development environment";

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
    pkgs = nixpkgs.legacyPackages.${system};
    pkgs-unstable = nixpkgs-unstable.legacyPackages.${system};
  in {
    devShells.${system}.default = pkgs.mkShell {
      packages = with pkgs; [
        # rustup supplies Cargo/rustc shims so installed Android targets are used.
        rustup
        nodejs_22
        pkg-config
        jdk17
        android-tools
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
        # Install the versioned SDK/NDK separately; see docs/android-build.md.
        # Respect an SDK selected by Android Studio or the caller.
        export ANDROID_HOME="''${ANDROID_HOME:-$HOME/Android/Sdk}"
        export NDK_HOME="''${NDK_HOME:-$ANDROID_HOME/ndk/29.0.14206865}"
      '';
    };
  };
}
