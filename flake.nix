{
  inputs.nixpkgs.url = github:NixOS/nixpkgs/nixos-unstable;

  outputs = { self, nixpkgs, fenix }:
    let
      name = "wayland-compositor-thing";
      pkgs = system: import nixpkgs {
        inherit system;
      };
      shell = pkgs: pkgs.mkShell {
        inputsFrom = [ self.packages.${pkgs.system}.default ];
        LD_LIBRARY_PATH = "${(pkgs.libGL.outPath + "/lib")}:${(pkgs.wayland.outPath + "/lib")}";
        shellHook = ''
          export PATH="${(pkgs.wlcs.outPath + "/libexec/wlcs")}:$PATH"
        '';
      };
      package = pkgs: let
        rpath = pkgs.stdenv.lib.makeLibraryPath [

        ];
      in pkgs.rustPlatform.buildRustPackage rec {
            pname = name;
            src = ./.;

            version = "0.0.1";

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            buildInputs = with pkgs; [
              wayland
              xorg.libX11
              xorg.libXcursor
              xorg.libXrandr
              xorg.libXi
              libxkbcommon
              libudev-zero
              libinput
              libdrm
              libGL
              libGLU
              mesa
            ];
          };
    in
    {
      overlays.default = final: prev: {
        wayland-compositor-thing = (prev.wayland-compositor-thing or {}) // {
          ${name} = package final;
        };
      };

      packages."x86_64-linux".default = package (pkgs "x86_64-linux");
      packages."aarch64-linux".default = package (pkgs "aarch64-linux");

      devShells."x86_64-linux".default = shell (pkgs "x86_64-linux");
      devShells."aarch64-linux".default = shell (pkgs "aarch64-linux");
    };
}
