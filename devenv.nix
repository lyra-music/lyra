{
  pkgs,
  lib,
  config,
  inputs,
  ...
}: let
  configEnv = config.secretspec.secrets;
  lavalinkDir = "./lavalink"; # use `config.git.root` on devenv 1.10
  lavalinkJar = "Lavalink.jar";
in {
  # https://devenv.sh/basics/
  # env.GREET = "devenv";

  # https://devenv.sh/packages/
  # packages = [ pkgs.git ];
  packages = with pkgs; [codespell cargo-edit];

  # https://devenv.sh/languages/
  # languages.rust.enable = true;
  languages = {
    java = {
      enable = true;
      jdk.package = pkgs.jdk21;
    };
    rust = {
      enable = true;
      channel = "stable";
      mold.enable = true;
    };
  };

  # https://devenv.sh/processes/
  # processes.dev.exec = "${lib.getExe pkgs.watchexec} -n -- ls -la";
  processes.lavalink = {
    exec = let
      scriptName = "lavalink";
      script = pkgs.writeShellScript scriptName "java -jar ${lavalinkJar}";
    in
      script.outPath;
    cwd = lavalinkDir;
    process-compose = {
      environment =
        ["_JAVA_OPTIONS=-Xmx6G"]
        ++ (
          let
            defaults = {
              LOGGING_LEVEL_ROOT = "WARN";
              LOGGING_LEVEL_LAVALINK = "WARN";
              PLUGINS_YOUTUBE_OAUTH_ENABLED = "false";
              PLUGINS_LAVASRC_SOURCES_SPOTIFY = "false";
              PLUGINS_LAVASRC_SOURCES_DEEZER = "false";
            };

            val = name:
              if builtins.hasAttr name configEnv
              then builtins.getAttr name configEnv
              else defaults.${name} or (throw "Missing required env var ${name}");

            toStr = v:
              if builtins.isBool v
              then
                (
                  if v
                  then "true"
                  else "false"
                )
              else builtins.toString v;

            asBool = v:
              if builtins.isBool v
              then v
              else v == "true";

            youtubeEnabled = asBool (val "PLUGINS_YOUTUBE_OAUTH_ENABLED");
            spotifyEnabled = asBool (val "PLUGINS_LAVASRC_SOURCES_SPOTIFY");
            deezerEnabled = asBool (val "PLUGINS_LAVASRC_SOURCES_DEEZER");

            alwaysRequired = [
              "SERVER_ADDRESS"
              "SERVER_PORT"
              "LAVALINK_SERVER_PASSWORD"
            ];

            withDefaults = builtins.attrNames defaults;

            conditionalRequired =
              (
                if youtubeEnabled
                then ["PLUGINS_YOUTUBE_OAUTH_REFRESH_TOKEN"]
                else []
              )
              ++ (
                if spotifyEnabled
                then ["PLUGINS_LAVASRC_SPOTIFY_CLIENT_ID" "PLUGINS_LAVASRC_SPOTIFY_CLIENT_SECRET"]
                else []
              )
              ++ (
                if deezerEnabled
                then ["PLUGINS_LAVASRC_DEEZER_MASTER_DECRYPTION_KEY" "PLUGINS_LAVASRC_DEEZER_ARL"]
                else []
              );

            emit = name: "${name}=${toStr (val name)}";
          in
            (map emit withDefaults)
            ++ (map emit alwaysRequired)
            ++ (map emit conditionalRequired)
        );

      readiness_probe = {
        http_get = {
          host = configEnv.SERVER_ADDRESS;
          port = configEnv.SERVER_PORT;
          path = "/version";
          headers = {
            Authorization = configEnv.LAVALINK_SERVER_PASSWORD;
          };
        };
        period_seconds = 60;
        initial_delay_seconds = 15;
        timeout_seconds = 10;
        failure_threshold = 5;
      };
    };
  };

  # https://devenv.sh/services/
  # services.postgres.enable = true;
  services.postgres = {
    enable = true;
    package = pkgs.postgresql_17;
    initialDatabases = [
      {
        name = configEnv.POSTGRES_DB;
        user = configEnv.POSTGRES_USER;
        pass = configEnv.POSTGRES_PASSWORD;
      }
    ];
    listen_addresses = configEnv.POSTGRES_HOST;
    port = lib.toInt configEnv.POSTGRES_PORT;
  };

  # https://devenv.sh/scripts/
  # scripts.hello.exec = ''
  #   echo hello from $GREET
  # '';

  # https://devenv.sh/basics/
  # enterShell = ''
  #   hello         # Run scripts directly
  #   git --version # Use packages
  # '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };
  tasks = {
    "lavalink:update" = {
      exec = let
        scriptName = "lavalink-update";
        script = pkgs.writeShellScript scriptName ''
          META_FILE="version.txt"
          FILE="${lavalinkJar}"

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
        '';
      in
        script.outPath;
      cwd = lavalinkDir;
      before = ["devenv:processes:lavalink"];
    };
  };

  # https://devenv.sh/tests/
  # enterTest = ''
  #   echo "Running tests"
  #   git --version | grep --color=auto "${pkgs.git.version}"
  # '';

  # https://devenv.sh/git-hooks/
  # git-hooks.hooks.shellcheck.enable = true;

  # See full reference at https://devenv.sh/reference/options/

  dotenv.disableHint = true; # already managed by secretspec
}
