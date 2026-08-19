{
  pkgs,
  lib,
  ...
}:
let
  darwinRuntimeLibraries = with pkgs; [
    libiconv
  ];
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
      pkgs.diesel-cli
      pkgs.nixfmt
    ]
    ++ lib.optionals pkgs.stdenv.isDarwin darwinRuntimeLibraries;

    git-hooks.hooks = {
      ty = {
        enable = true;
        entry = "uv run --all-extras ty check";
        pass_filenames = false;
      };
      ruff.enable = true;
      ruff-format.enable = true;
      git-hooks.package = pkgs.prek;
      typos.enable = true;
    };

    # https://devenv.sh/languages/
    languages.python = {
      enable = true;
      package = pkgs.python313;
      uv = {
        enable = true;
        sync.enable = true;
      };
    };

    languages.rust = {

      enable = true;
      components = [
        "rustc"
        "cargo"
        "clippy"
        "rustfmt"
        "rust-analyzer"
      ];
    };

    languages.javascript = {
      enable = true;
      pnpm.enable = true;
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
      if pkgs.stdenv.isDarwin && pkgs.stdenv.isAarch64 then
        ''
          unset UV_PYTHON;
          export MATURIN_NO_PROGRESS=1
          export RUST_LOG=error
        ''
      else
        ''
          export LD_LIBRARY_PATH=${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.zlib}/lib:$LD_LIBRARY_PATH
          export MATURIN_NO_PROGRESS=1
          export RUST_LOG=error
        '';
  };
}
