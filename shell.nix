{ pkgs ? import <nixpkgs> { } }:
with pkgs;
mkShell {
  buildInputs = [
    cargo rustc gcc
    wireguard-tools
    glib
    gtk3
    pkg-config
    # libindicator-gtk3
    # libappindicator-gtk3
    # libayatana-appindicator
  ];

  # shellHook = ''
  #   # ...
  # '';
}
