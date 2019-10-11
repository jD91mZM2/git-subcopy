{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  # Things to be put in $PATH
  nativeBuildInputs = with pkgs; [ pkg-config ];

  # Libraries to be installed
  buildInputs = with pkgs; [ openssl ];
}
