{ essential-rest-server
, nixos-generators
, pkgs
, system
}:
let
  base-config = {
    system.stateVersion = "23.11";
    networking.firewall.allowedTCPPorts = [ 80 22 ];
    networking.firewall.enable = true;
    systemd.services.essential-server = {
      enable = true;
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      script = ''
        ${essential-rest-server}/bin/essential-rest-server 0.0.0.0:80 --rqlite-address="rqlite.essential.internal:4001"
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

    services.openssh = {
      enable = true;
      settings.PasswordAuthentication = false;
    };
    nix.settings.experimental-features = [ "nix-command" "flakes" ];
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
  image-name = "essential-${system}";
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
#       virtualisation.diskImage = "essential-server";
#       virtualisation.forwardPorts = [
#         { from = "host"; host.port = 3080; guest.port = 80; }
#         { from = "host"; host.port = 3022; guest.port = 22; }
#       ];
#     }
#   ];
# }
