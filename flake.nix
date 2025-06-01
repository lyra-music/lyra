{
  inputs = {
    nixpkgs.url = "github:cachix/devenv-nixpkgs/rolling";
    systems.url = "github:nix-systems/default";
    devenv = {
      url = "github:cachix/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = {
    self,
    nixpkgs,
    devenv,
    systems,
    ...
  } @ inputs: let
    forEachSystem = nixpkgs.lib.genAttrs (import systems);
  in {
    packages = forEachSystem (system: {
      devenv-up = self.devShells.${system}.default.config.procfileScript;
    });

    devShells =
      forEachSystem
      (system: let
        pkgs = nixpkgs.legacyPackages.${system};
      in {
        default = devenv.lib.mkShell {
          inherit inputs pkgs;
          modules = [
            {
              # https://devenv.sh/basics/
              # env.GREET = "devenv";

              # https://devenv.sh/packages/
              packages = with pkgs; [
                codespell
                cargo-edit
                #sqlx-cli
                #clang
                #mold
                #act
                #pgcli
              ];

              # https://devenv.sh/scripts/
              # scripts.hello.exec = "";

              # enterShell = ''
              # '';

              # https://devenv.sh/tests/
              # enterTest = ''
              # '';

              # https://devenv.sh/services/
              services.postgres = {
                enable = true;
                package = pkgs.postgresql_17;
                initialDatabases = [
                  {
                    name = "lyra";
                    user = "lyra";
                    pass = "correcthorsebatterystaple"; # only serve as an initial password;
                    # don't forget to change the actual user password to `POSTGRES_PASSWORD`: `ALTER USER lyra WITH PASSWORD 'new_password';`
                  }
                ];
                listen_addresses = "localhost";
              };

              # https://devenv.sh/languages/
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

              # Enable Codespaces Integration
              # https://devenv.sh/integrations/codespaces-devcontainer/
              devcontainer.enable = true;

              # https://devenv.sh/pre-commit-hooks/
              # pre-commit.hooks.shellcheck.enable = true;

              # https://devenv.sh/processes/
              processes.lavalink = {
                #process-compose = {};
                exec = ''
                  LAVALINK_DIR="$PWD/lavalink"
                  mkdir -p "$LAVALINK_DIR"

                  META_FILE="$LAVALINK_DIR/version.txt"
                  FILE="$LAVALINK_DIR/Lavalink.jar"

                  # Get the latest Lavalink release metadata from GitHub API
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

                  set -a # automatically export all variables
                  source .env
                  set +a

                  cd lavalink && java -jar Lavalink.jar
                '';
              };

              # See full reference at https://devenv.sh/reference/options/
              # dotenv.enable = true;
            }
          ];
        };
      });
  };
}
