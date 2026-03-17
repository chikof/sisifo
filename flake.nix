{
  inputs = {
    flakelight-rust.url = "github:accelbread/flakelight-rust";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = {
    flakelight-rust,
    nixpkgs,
    ...
  }:
    flakelight-rust ./. {
      devShell.packages = pkgs:
        with pkgs; [
          pkg-config
          openssl
          glib
          gtk3
          webkitgtk_4_1
          libsoup_3
          cairo
          pango
          gdk-pixbuf
          librsvg
          dbus
          cargo-tauri
          cmake
          nodejs
          pnpm
        ];

      devShell.env = pkgs: {
        PKG_CONFIG_PATH = pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" (with pkgs; [
          webkitgtk_4_1
          gtk3
          glib
          openssl
          libsoup_3
        ]);
      };
    };
}
