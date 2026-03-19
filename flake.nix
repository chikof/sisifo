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
          gsettings-desktop-schemas
          iroh-relay
          iroh-dns-server
        ];

      devShell.env = pkgs: {
        PKG_CONFIG_PATH = pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" (with pkgs; [
          webkitgtk_4_1
          gtk3
          glib
          openssl
          libsoup_3
        ]);

        WEBKIT_DISABLE_COMPOSITING_MODE = "1";
        GDK_BACKEND = "x11";

        GSETTINGS_SCHEMA_DIR = pkgs.lib.concatStringsSep ":" [
          "${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}/glib-2.0/schemas"
          "${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}/glib-2.0/schemas"
        ];
      };
    };
}
