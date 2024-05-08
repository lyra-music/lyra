{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  name = "dev";

  nativeBuildInputs = with pkgs; [
    clang
    mold
    pgcli
    sqlx-cli
    postgresql
    zulu
  ];

  LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib}/lib";

  shellHook = ''
    TMPD="$PWD/.tmp"
    export PGDATA="$TMPD/mydb"
    export SOCKET_DIRECTORIES="$TMPD/socket"

    mkdir -p $SOCKET_DIRECTORIES

    initdb
    echo "unix_socket_directories = '$SOCKET_DIRECTORIES'" >> $PGDATA/postgresql.conf
    pg_ctl -l $PGDATA/logfile start
    createuser postgres --createdb -h localhost
    ./scripts/db

    function end {
      pg_ctl stop
      rm -rf $TMPD
    }
    trap end EXIT
  '';
}
