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
        pkgs = import nixpkgs { inherit system; };
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
            pkgs.pkg-config
          ];

          buildInputs = [
            # Libraries
            pkgs.glfw
            pkgs.glib
            pkgs.cglm
            pkgs.pcre2
            pkgs.libsysprof-capture
            pkgs.assimp
            wasserxr.packages.${system}.default
          ];

          cmakeFlags = [
            (lib.cmakeBool "BUILD_DEBUG" false)
            (lib.cmakeBool "WXR_STATIC" false)
            (lib.cmakeBool "WXR_TESTS" false)
          ];

          meta = {
            license = pkgs.lib.licenses.mit;
          };
        };
        checks = {
          clang-tidy = self.packages.${system}.default.overrideAttrs (oldAttrs: {
            pname = "WasserXR-Core-clang-tidy";
            nativeBuildInputs = oldAttrs.nativeBuildInputs ++ [ pkgs.python3 ];
            cmakeFlags = [
              (lib.cmakeBool "BUILD_DEBUG" false)
              (lib.cmakeBool "WXR_STATIC" false)
              (lib.cmakeBool "WXR_TESTS" false)
            ];
            doCheck = true;
            checkPhase = ''
              runHook preCheck
              python3 $(command -v run-clang-tidy) -p . -warnings-as-errors='*' 'src/WasserXR/.*\.c$'
              runHook postCheck
            '';
          });
          default = self.packages.${system}.default.overrideAttrs (_: {
            pname = "WasserXR-Core-tests";
            cmakeFlags = [
              (lib.cmakeBool "BUILD_DEBUG" false)
              (lib.cmakeBool "WXR_STATIC" false)
              (lib.cmakeBool "WXR_TESTS" true)
            ];
            doCheck = true;
            checkPhase = ''
              runHook preCheck
              ctest --output-on-failure
              runHook postCheck
            '';
          });
        };
        devShells.default = pkgs.mkShell {
          name = "devShell";

          buildInputs = [
            pkgs.clang-tools
            pkgs.clang
            pkgs.cmake
            pkgs.gdb
            pkgs.valgrind

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
