{ pkgs, lib, ... }:

{

  packages = [
    pkgs.just
    pkgs.maturin
    pkgs.graphviz
    pkgs.zlib
  ]
  ++ lib.optionals pkgs.stdenv.isDarwin (
    with pkgs.darwin.apple_sdk;
    [
      frameworks.CoreServices
      frameworks.CoreFoundation
      frameworks.Security
      frameworks.SystemConfiguration
    ]
  );

  git-hooks.hooks = {
    pyright.enable = true;
    ruff.enable = true;
    ruff-format.enable = true;
  };

  # https://devenv.sh/languages/
  languages.python = {
    enable = true;
    uv.enable = true;
  };

  languages.rust.enable = true;

  languages.javascript = {
    enable = true;
  };

  # This allows building the type-check (pyo3) module on MacOSX "Apple Silicon"
  enterShell =
    if pkgs.stdenv.isDarwin && pkgs.stdenv.isAarch64 then
      ''
        export RUSTFLAGS="$RUSTFLAGS -C link-arg=-undefined -C link-arg=dynamic_lookup"
      ''
    else
      '''';

}
