_: {
  projectRootFile = "flake.nix";

  # rust files are formatted by `cargo fmt` (checks.fmt and the pre-commit hook),
  # so rustfmt is not listed here and cannot disagree about the edition
  programs = {
    nixfmt.enable = true;
    statix.enable = true;
    deadnix.enable = true;
    taplo.enable = true;
  };
}
