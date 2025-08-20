{
  description = "Development environment for Λύρα";

  inputs = {
    devenv-root = {
      url = "file+file:///dev/null";
      flake = false;
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";

    devenv.url = "github:cachix/devenv";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Optional tools for future extensibility
    nix2container = {
      url = "github:nlewo/nix2container";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    mk-shell-bin.url = "github:rrbutani/nix-mk-shell-bin";
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = inputs @ {
    flake-parts,
    devenv,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [
        inputs.devenv.flakeModule
      ];

      # Supported systems (from nix-systems/default)
      systems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];

      perSystem = {
        system,
        pkgs,
        config,
        inputs',
        ...
      }: {
        # devenv shell configuration
        devenv.shells.default = {
          name = "main";

          # Add packages from nixpkgs
          packages = with pkgs; [
            codespell
            cargo-edit
          ];

          # Services configuration
          services.postgres = {
            enable = true;
            package = pkgs.postgresql_17;
            initialDatabases = [
              {
                name = "lyra";
                user = "lyra";
                pass = "correcthorsebatterystaple";
              }
            ];
            listen_addresses = "localhost";
          };

          # Languages
          languages.nix.enable = true;
          languages.java = {
            enable = true;
            jdk.package = pkgs.jdk17;
          };
          languages.rust = {
            enable = true;
            channel = "stable";
            mold.enable = true;
          };

          # Enable Codespaces devcontainer support
          devcontainer.enable = true;

          # Custom processes
          processes.lavalink.exec = ''
            LAVALINK_DIR="$PWD/lavalink"
            mkdir -p "$LAVALINK_DIR"

            META_FILE="$LAVALINK_DIR/version.txt"
            FILE="$LAVALINK_DIR/Lavalink.jar"

            LATEST_JSON=$(curl -s https://api.github.com/repos/lavalink-devs/Lavalink/releases/latest)
            LATEST_VERSION=$(echo "$LATEST_JSON" | grep '"tag_name":' | cut -d '"' -f 4)
            LATEST_URL=$(echo "$LATEST_JSON" | grep "browser_download_url" | grep "Lavalink.jar" | cut -d '"' -f 4)

            NEED_DOWNLOAD=1

            if [ -f "$META_FILE" ]; then
              CURRENT_VERSION=$(cat "$META_FILE")
              if [ "$CURRENT_VERSION" = "$LATEST_VERSION" ] && [ -f "$FILE" ]; then
                echo "Lavalink is up to date (version $CURRENT_VERSION)."
                NEED_DOWNLOAD=0
              fi
            fi

            if [ "$NEED_DOWNLOAD" -eq 1 ]; then
              echo "Downloading Lavalink $LATEST_VERSION..."
              curl -Lo "$FILE" "$LATEST_URL"
              echo "$LATEST_VERSION" > "$META_FILE"
            fi

            set -a
            source .env
            set +a

            cd lavalink && java -jar Lavalink.jar
          '';
        };
      };

      flake = {
        # Here you can define additional flake outputs like overlays, templates, etc.
      };
    };
}
