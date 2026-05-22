{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    wasserxr.url = "/home/lars/GitHub/WasserXR/WasserXR";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      wasserxr,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = nixpkgs.lib;
      in
      {
        packages.default = pkgs.stdenv.mkDerivation {
          pname = "WasserXR-Core";
          version = "pre 0.1.0";

          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter =
              path: type:
              let
                baseName = builtins.baseNameOf path;
              in
              !(baseName == "build");
          };

          nativeBuildInputs = [
            # Build packages
            pkgs.clang-tools
            pkgs.clang
            pkgs.cmake
            pkgs.doxygen

            # Libraries
            pkgs.glfw
            pkgs.glib
            pkgs.cglm
            pkgs.pkg-config
            pkgs.pcre2
            pkgs.libsysprof-capture
            pkgs.assimp
            wasserxr.packages.${system}.default
          ];

          cmakeFlags = [
            (lib.cmakeBool "BUILD_DEBUG" true)
            (lib.cmakeBool "WXR_STATIC" false)
            (lib.cmakeBool "WXR_TESTS" false)
          ];

          meta = {
            license = pkgs.lib.licenses.mit;
          };
        };
        devShells.default = pkgs.mkShell {
          name = "devShell";

          buildInputs = [
            pkgs.clang-tools
            pkgs.clang
            pkgs.cmake
            pkgs.gdb

            pkgs.doxygen

            # Libraries
            pkgs.glfw
            pkgs.glib
            pkgs.cglm
            pkgs.pkg-config
            pkgs.pcre2
            pkgs.libsysprof-capture
            pkgs.assimp
            wasserxr.packages.${system}.default
          ];

          shellHook = ''
            export ASAN_SYMBOLIZER_PATH="${pkgs.llvm}/bin/llvm-symbolizer"

            export ASAN_OPTIONS="symbolize=1:check_initialization_order=1:detect_stack_use_after_return=1:strict_string_checks=1:detect_leaks=1"
            export UBSAN_OPTIONS="print_stacktrace=1:halt_on_error=0"
          '';
        };
      }
    );
}
