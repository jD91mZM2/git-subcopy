{ pkgs ? import ./pinned.nix {} }:

(pkgs.callPackage ./Cargo.nix {}).rootCrate.build
