{
    description = "mdor —— 移动端 mdBook 离线阅读器（Rust + Dioxus）开发环境";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "nixpkgs";
        };
        flake-utils.url = "github:numtide/flake-utils";

        treefmt-nix = {
            url = "github:numtide/treefmt-nix";
            inputs.nixpkgs.follows = "nixpkgs";
        };
    };

    outputs = {
        self,
        nixpkgs,
        rust-overlay,
        flake-utils,
        treefmt-nix,
        ...
    }:
        flake-utils.lib.eachDefaultSystem (
            system: let
                overlays = [(import rust-overlay)];
                pkgs = import nixpkgs {
                    inherit system overlays;
                };
                rustToolchain = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
                    extensions = [
                        "rust-src"
                        "rust-analyzer"
                        "rustfmt"
                        "clippy"
                    ];
                };

                treefmt = treefmt-nix.lib.mkWrapper pkgs {
                    projectRootFile = "flake.nix";
                    programs = {
                        rustfmt = {
                            enable = true;
                            package = rustToolchain;
                        };
                        alejandra.enable = true;
                    };
                };
            in {
                devShells.default = pkgs.mkShell {
                    # nativeBuildInputs 是给编译机（开发用来编译软件的平台）的依赖
                    nativeBuildInputs = [
                        rustToolchain
                        pkgs.pkg-config
                        pkgs.cargo-audit
                        pkgs.cargo-outdated
                        pkgs.cargo-edit
                    ];
                    # buildInputs 是给编译目标平台（编译完软件运行的平台）所需要的依赖
                    buildInputs = [
                        pkgs.openssl # 这里声明库依赖
                        pkgs.webkitgtk_4_1
                        pkgs.gtk3
                        pkgs.libsoup_3
                        pkgs.gdk-pixbuf
                        pkgs.xdotool
                    ];
                    shellHook = ''
                        if [ ! -x "$HOME/.cargo/bin/dx" ] || ! "$HOME/.cargo/bin/dx" --version 2>/dev/null | grep -q "0.7.10"; then
                            cargo install dioxus-cli --locked --version 0.7.10
                        fi
                        export PATH="$HOME/.cargo/bin:$PATH"
                    '';
                };

                # 🆕 使用 rustfmt 作为 formatter
                formatter = treefmt;
            }
        );
}
