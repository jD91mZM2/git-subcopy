# Used by default.nix in case no nixpkgs is specified. Pinning is
# useful to ensure cachix binary cache gets used.

import (builtins.fetchGit {
  name = "nixos-19.09-2019-10-27";
  url = https://github.com/nixos/nixpkgs/;
  # Commit hash for nixos-unstable as of 2019-10-27
  # `git ls-remote https://github.com/nixos/nixpkgs-channels nixos-19.09`
  ref = "refs/heads/nixos-19.09";
  rev = "27a5ddcf747fb2bb81ea9c63f63f2eb3eec7a2ec";
})
