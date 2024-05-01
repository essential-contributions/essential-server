{ rqlite
, nixos-generators
, pkgs
}:
let
  base-config = {
    system.stateVersion = "23.11";
    networking.firewall.allowedTCPPorts = [ 4001 ];
    systemd.services.iplz = {
      enable = true;
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      script = ''
        ${rqlite} --node-id=1 data
      '';
      serviceConfig = {
        Restart = "always";
        Type = "simple";
      };
    };
  };
in
nixos-generators.nixosGenerate {
  inherit pkgs;
  format = "vm";
  modules = [
    base-config
    {
      services.getty.autologinUser = "root";
      virtualisation.forwardPorts = [{ from = "host"; host.port = 4001; guest.port = 4001; }];
    }
  ];
}
