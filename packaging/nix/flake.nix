{
  description = "Sonar: a Dolphin-inspired file manager (Tauri 2 + Rust)";

  inputs.nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0";

  outputs =
    { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;

      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      forEachSupportedSystem =
        f: lib.genAttrs supportedSystems (system: f nixpkgs.legacyPackages.${system});

      cargoToml = lib.importTOML ./Cargo.toml;
    in
    {
      packages = forEachSupportedSystem (pkgs: {
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.sonar;

        sonar = pkgs.rustPlatform.buildRustPackage {
          pname = "sonar";
          inherit (cargoToml.workspace.package) version;

          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./crates
              ./src-tauri
              ./ui
            ];
          };

          cargoLock.lockFile = ./Cargo.lock;

          # Same as the AUR build: plain cargo, with the UI embedded the way `tauri build` does it.
          cargoBuildFlags = [
            "-p"
            "sonar"
            "--features"
            "tauri/custom-protocol"
          ];
          # Tests touch the real desktop (pasteboard, trash, mime db); run them via `nix develop` instead.
          doCheck = false;

          nativeBuildInputs =
            with pkgs;
            [ pkg-config ] ++ lib.optionals stdenv.hostPlatform.isLinux [ wrapGAppsHook3 ];

          buildInputs = lib.optionals pkgs.stdenv.hostPlatform.isLinux (
            with pkgs;
            [
              webkitgtk_4_1
              gtk3
              glib-networking # TLS for WebKit
              openssl
            ]
          );

          # Tools Sonar shells out to (archives, Open With, mounting).
          preFixup = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            gappsWrapperArgs+=(
              --suffix PATH : ${
                lib.makeBinPath (
                  with pkgs;
                  [
                    libarchive
                    xdg-utils
                    util-linux
                  ]
                )
              }
              --suffix XDG_DATA_DIRS : ${pkgs.shared-mime-info}/share:${pkgs.hicolor-icon-theme}/share
            )
          '';

          postInstall = ''
            # The cdylib/staticlib crate types exist for mobile targets; the binary doesn't need them.
            rm -rf $out/lib
          ''
          + lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            sed -e 's/{{name}}/Sonar/' -e 's/{{exec}}/sonar/g' -e 's/{{icon}}/sonar/' \
              src-tauri/linux/sonar.desktop > sonar.desktop
            install -Dm644 sonar.desktop $out/share/applications/sonar.desktop
            install -Dm644 src-tauri/icons/32x32.png $out/share/icons/hicolor/32x32/apps/sonar.png
            install -Dm644 src-tauri/icons/128x128.png $out/share/icons/hicolor/128x128/apps/sonar.png
            install -Dm644 src-tauri/icons/128x128@2x.png $out/share/icons/hicolor/256x256/apps/sonar.png
            install -Dm644 src-tauri/icons/icon.png $out/share/icons/hicolor/512x512/apps/sonar.png
            install -Dm644 src-tauri/icons/icon.svg $out/share/icons/hicolor/scalable/apps/sonar.svg
          '';

          meta = {
            description = "A Dolphin-inspired file manager";
            homepage = "https://github.com/samtoxie/sonar";
            license = lib.licenses.eupl12;
            mainProgram = "sonar";
            platforms = supportedSystems;
          };
        };
      });

      apps = forEachSupportedSystem (pkgs: {
        default = {
          type = "app";
          program = lib.getExe self.packages.${pkgs.stdenv.hostPlatform.system}.sonar;
        };
      });

      devShells = forEachSupportedSystem (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.sonar ];
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
            cargo-tauri
            nodejs
            libarchive
            self.formatter.${pkgs.stdenv.hostPlatform.system}
          ];
        };
      });

      formatter = forEachSupportedSystem (pkgs: pkgs.nixfmt);
    };
}
