# AGENTS instructions

## Dev environment tips
- Most common operations have scripts available in the root `justfile`.
- When `devenv` is installed, locked versions of tools can be provided from `devenv.nix`.
- Sometimes it may be necessary to update generated code with `just generate`.

## Testing instructions
- Run `just lint` and `just test` to run linting and testing for python and rust.
- `just fix` can be used to automatically fix many lint errors.
- Prefer parametrized tests using pytest or rstest.
- When possible use property based testing libraries `hypothesis` and `proptest` to
  find edge cases in unit tests.
- Use pyright to find type errors. When writing new code use strict checking.
- Try not to encode implicit behavior into unit tests unless asked by the user.
