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
                git
                clang
                mold
                codespell
                act
                sqlx-cli
                pgcli
              ];

              # https://devenv.sh/scripts/
              # scripts.hello.exec = "";

              enterShell = ''
              '';

              # https://devenv.sh/tests/
              enterTest = ''
              '';

              # https://devenv.sh/services/
              services.postgres = {
                enable = true;
                package = pkgs.postgresql_15;
                initialDatabases = [{name = "mydb";}];
                extensions = extensions: [
                  extensions.postgis
                  extensions.timescaledb
                ];
                settings.shared_preload_libraries = "timescaledb";
                initialScript = "CREATE EXTENSION IF NOT EXISTS timescaledb;";
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
              processes.lavalink.exec = "scripts/lavalink";

              processes.database-setup.exec = ''
                while ! "./scripts/database"
                do
                  :
                done
              '';

              # See full reference at https://devenv.sh/reference/options/
              # dotenv.enable = true;
            }
          ];
        };
      });
  };
}
