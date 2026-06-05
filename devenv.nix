{ pkgs, lib, inputs, config, pkgsHostHost, ... }:
let
  darwinRuntimeLibraries = with pkgs; [
    libiconv
  ];
  darwinRuntimeLibraryPath = lib.makeLibraryPath darwinRuntimeLibraries;
in
{
 config = {
  packages = [
    pkgs.just
    pkgs.zlib
    pkgs.maturin
    pkgs.bacon
    pkgs.cargo-nextest
    pkgs.sqlite
  ] ++ lib.optionals pkgs.stdenv.isDarwin darwinRuntimeLibraries;

  git-hooks.hooks = {
    pyright.enable = true;
    ruff.enable = true;
    ruff-format.enable = true;
    git-hooks.package = pkgs.prek;
    typos.enable = true;
  };

  # https://devenv.sh/languages/
  languages.python = {
    enable = true;
     uv = {
        enable = true;
        sync.enable = true;
      };
  };

    languages.rust = {

      enable = true;
      components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
    };

  languages.javascript = {
    enable = true;
    npm.enable = true;
    directory = "./tierkreis_visualization";
  };
  scripts.sbatch.exec = ''
    ./infra/slurm_local/sbatch "$@";
  '';
  scripts.squeue.exec = ''
    ./infra/slurm_local/squeue "$@";
  '';
  scripts.qsub.exec = ''
    ./infra/pbs_local/qsub "$@";
  '';

  # This allows building the type-check (pyo3) module on MacOSX "Apple Silicon"
  enterShell =
    if pkgs.stdenv.isDarwin && pkgs.stdenv.isAarch64 then ''
      unset UV_PYTHON;
      export MATURIN_NO_PROGRESS=1
      export RUST_LOG=error
    '' else ''
      export MATURIN_NO_PROGRESS=1
      export RUST_LOG=error
    '';
 };
}
