{ pkgs, lib, ... }:

{

  packages = [
    pkgs.just
    pkgs.zlib
    pkgs.maturin
    pkgs.bacon
    pkgs.cargo-nextest
  ] ++ lib.optionals pkgs.stdenv.isDarwin [
    pkgs.apple-sdk
  ];

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
    uv.enable = true;
  };

  languages.rust.enable = true;

  languages.javascript = {
    enable = true;
    npm.enable = true;
    directory = "./tierkreis_visualization";
  };

  # This allows building the type-check (pyo3) module on MacOSX "Apple Silicon"
  enterShell =
    if pkgs.stdenv.isDarwin && pkgs.stdenv.isAarch64 then ''
      export RUSTFLAGS="$RUSTFLAGS -C link-arg=-undefined -C link-arg=dynamic_lookup"
    '' else '''';

}
