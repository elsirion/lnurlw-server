{
  description = "LNURLw server for Bolt Cards";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    let
      # NixOS module for the lnurlw server
      nixosModule = { config, lib, pkgs, ... }:
        with lib;
        let
          cfg = config.services.lnurlw-server;
          
          # Build the lnurlw-server package
          lnurlw-server = pkgs.rustPlatform.buildRustPackage {
            pname = "lnurlw-server";
            version = "0.1.0";
            
            src = self;
            
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            
            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
            
            buildInputs = with pkgs; [
              openssl
              sqlite
            ];
          };
        in
        {
          options.services.lnurlw-server = {
            enable = mkEnableOption "LNURLw server for Bolt Cards";
            
            domain = mkOption {
              type = types.str;
              description = "Domain name for the LNURLw server";
              example = "cards.example.com";
            };
            
            host = mkOption {
              type = types.str;
              default = "127.0.0.1";
              description = "Host address to bind the server to";
            };
            
            port = mkOption {
              type = types.port;
              default = 3000;
              description = "Port for the LNURLw server";
            };
            
            databaseUrl = mkOption {
              type = types.str;
              default = "sqlite:/var/lib/lnurlw-server/lnurlw.db";
              description = "Database URL for SQLite";
            };
            
            defaultTxLimit = mkOption {
              type = types.int;
              default = 50000;
              description = "Default per-transaction limit in satoshis";
            };
            
            defaultDayLimit = mkOption {
              type = types.int;
              default = 500000;
              description = "Default daily spending limit in satoshis";
            };
            
            nginx = {
              enable = mkOption {
                type = types.bool;
                default = true;
                description = "Enable nginx reverse proxy";
              };
              
              enableSSL = mkOption {
                type = types.bool;
                default = true;
                description = "Enable SSL/TLS with Let's Encrypt";
              };
              
              forceSSL = mkOption {
                type = types.bool;
                default = true;
                description = "Force SSL (redirect HTTP to HTTPS)";
              };
            };
            
            user = mkOption {
              type = types.str;
              default = "lnurlw";
              description = "User to run the service as";
            };
            
            group = mkOption {
              type = types.str;
              default = "lnurlw";
              description = "Group to run the service as";
            };
          };
          
          config = mkIf cfg.enable {
            # Create system user and group
            users.users.${cfg.user} = {
              isSystemUser = true;
              group = cfg.group;
              home = "/var/lib/lnurlw-server";
              createHome = true;
            };
            
            users.groups.${cfg.group} = {};
            
            # Systemd service
            systemd.services.lnurlw-server = {
              description = "LNURLw server for Bolt Cards";
              after = [ "network.target" ];
              wantedBy = [ "multi-user.target" ];
              
              environment = {
                DOMAIN = cfg.domain;
                HOST = cfg.host;
                PORT = toString cfg.port;
                DATABASE_URL = cfg.databaseUrl;
                DEFAULT_TX_LIMIT = toString cfg.defaultTxLimit;
                DEFAULT_DAY_LIMIT = toString cfg.defaultDayLimit;
                RUST_LOG = "info";
              };
              
              serviceConfig = {
                Type = "simple";
                User = cfg.user;
                Group = cfg.group;
                ExecStart = "${lnurlw-server}/bin/lnurlw-server";
                Restart = "always";
                RestartSec = 10;
                
                # Security hardening
                PrivateTmp = true;
                NoNewPrivileges = true;
                ProtectSystem = "strict";
                ProtectHome = true;
                ReadWritePaths = [ "/var/lib/lnurlw-server" ];
                
                # Ensure database directory permissions
                StateDirectory = "lnurlw-server";
                StateDirectoryMode = "0750";
              };
              
              preStart = ''
                # Ensure database directory exists
                mkdir -p /var/lib/lnurlw-server
                chown ${cfg.user}:${cfg.group} /var/lib/lnurlw-server
              '';
            };
            
            # Nginx reverse proxy configuration
            services.nginx = mkIf cfg.nginx.enable {
              enable = true;
              
              virtualHosts.${cfg.domain} = {
                enableACME = cfg.nginx.enableSSL;
                forceSSL = cfg.nginx.forceSSL;
                
                locations."/" = {
                  proxyPass = "http://${cfg.host}:${toString cfg.port}";
                  proxyWebsockets = true;
                  
                  extraConfig = ''
                    proxy_set_header Host $host;
                    proxy_set_header X-Real-IP $remote_addr;
                    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
                    proxy_set_header X-Forwarded-Proto $scheme;
                    
                    # Timeouts for long-running requests
                    proxy_read_timeout 300s;
                    proxy_connect_timeout 75s;
                  '';
                };
              };
            };
            
            # Open firewall ports if nginx is enabled
            networking.firewall = mkIf cfg.nginx.enable {
              allowedTCPPorts = [ 80 443 ];
            };
          };
        };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "lnurlw-server";
          version = "0.1.0";
          
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          
          buildInputs = with pkgs; [
            openssl
            sqlite
          ];
        };
        
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo-watch
            cargo-edit
            pkg-config
            openssl
            sqlite
          ];

          shellHook = ''
            echo "LNURLw server development environment loaded"
            rustc --version
          '';

          RUST_BACKTRACE = 1;
        };
      }
    ) // {
      # NixOS module output
      nixosModules.default = nixosModule;
      nixosModules.lnurlw-server = nixosModule;
    };
}