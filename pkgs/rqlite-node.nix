{ rqlite
, nixos-generators
, pkgs
, system
}:
let
  base-config = {
    system.stateVersion = "23.11";
    networking.firewall.allowedTCPPorts = [ 4001 4002 22 ];
    networking.firewall.allowedUDPPorts = [ 4001 4002 ];
    networking.firewall.enable = true;
    systemd.services.rqlite-server = {
      enable = true;
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      script = ''
        PUBIP=$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4)
        INSTANCE_ID=$(curl http://169.254.169.254/latest/meta-data/instance-id)
        ${rqlite}/bin/rqlited -node-id="$INSTANCE_ID" -http-addr "0.0.0.0:4001" -http-adv-addr "$PUBIP:4001" -raft-addr "0.0.0.0:4002" -raft-adv-addr "$PUBIP:4002" -disco-key rqlite1 -disco-mode etcd-kv -disco-config "{\"endpoints\":[\"10.0.2.50:2379\"]}" data
      '';
      serviceConfig = {
        Restart = "always";
        Type = "simple";
      };
    };
    users.users.freesig = {
      isNormalUser = true;
      description = "freesig";
      extraGroups = [ "networkmanager" "wheel" ];
      openssh.authorizedKeys.keys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEHU9x9DkkVWBt9BzTZP/V6XtsUzvyQ72CjJZPxCrMAf tomrgowan@example.com" # content of authorized_keys file
      ];
      # initialPassword = "password";
    };
    nix.settings.experimental-features = [ "nix-command" "flakes" ];

    services.openssh = {
      enable = true;
      settings.PasswordAuthentication = false;
    };
    environment.noXlibs = true;
    documentation.enable = false;
    documentation.doc.enable = false;
    documentation.info.enable = false;
    documentation.man.enable = false;
    documentation.nixos.enable = false;
    environment.defaultPackages = [ ];
    environment.stub-ld.enable = false;
    programs.less.lessopen = null;
    boot.enableContainers = false;
    programs.command-not-found.enable = false;
    services.logrotate.enable = false;
    services.udisks2.enable = false;
    xdg.autostart.enable = false;
    xdg.icons.enable = false;
    xdg.mime.enable = false;
    xdg.sounds.enable = false;
  };
  image-name = "rqlite-${system}";
in
nixos-generators.nixosGenerate {
  inherit pkgs;
  format = "amazon";
  modules = [
    base-config
    { amazonImage.name = image-name; }
  ];
}
# nixos-generators.nixosGenerate {
#   inherit pkgs;
#   format = "vm";
#   modules = [
#     base-config
#     {
#       # services.getty.autologinUser = "root";
#       virtualisation.diskImage = disk-image-name;
#       virtualisation.forwardPorts = [
#         # { from = "host"; host.port = 4001; guest.port = 4001; }
#         # { from = "host"; host.port = 3022; guest.port = 22; }
#         { from = "host"; host.port = port-to-forward; guest.port = 4001; }
#         { from = "host"; host.port = raft-port; guest.port = 4002; }
#         { from = "host"; host.port = ssh-port-to-forward; guest.port = 22; }
#       ];
#     }
#   ];
# }
