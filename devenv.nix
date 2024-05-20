{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: {
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
    git --version
  '';

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    git --version | grep "2.42.0"
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

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # https://devenv.sh/processes/
  processes.lavalink.exec = "scripts/lavalink";

  # See full reference at https://devenv.sh/reference/options/
  dotenv.enable = true;
}
