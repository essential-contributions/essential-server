{ flake-utils
, pkgs
, rqlite
, essential-rest-server
}:
flake-utils.lib.mkApp {
  drv = pkgs.writeShellApplication {
    name = "server-with-rqlite";

    runtimeInputs = [ essential-rest-server rqlite ];

    text = ''
      # Check if an argument is provided
      if [ "$#" -eq 0 ]; then
        echo "Usage: $0 <rqlite-file-location>"
        exit 1
      fi

      # Start rqlite in the background
      rqlited -node-id=1 "$1" > /dev/null 2>&1 &
      pid=$!  # Capture the PID of the last job run in the background

      # Set a trap to kill the background process when the script exits
      trap 'kill $pid 2> /dev/null' EXIT

      # Wait for rqlite to start
      sleep 2

      essential-rest-server --db rqlite --rqlite-address http://localhost:4001

      # Wait for the background process to finish
      wait $pid
    '';
  };
}
