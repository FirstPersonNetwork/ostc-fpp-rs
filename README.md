# Linux Kernel Maintainer Verification

[![Rust](https://img.shields.io/badge/rust-1.88.0%2B-blue.svg?maxAge=3600)](https://github.com/FirstPersonNetwork/lkmv)

## Table of Contents

- [Core Concepts](#core-concepts)
- [Decentralised Identity](#decentralised-identity)
- [Decentralised Communication](#decentralised-communication)
- [Profiles and Configurations](#profiles-and-configurations)
  - [Public Configuration](#public-configuration)
  - [Private Configuration](#private-configuration)
  - [Secured Configuration](#secured-configuration)
- [Prerequisites](#prerequisites)
- [Feature Flags](#feature-flags)
- [Set Up Environment](#set-up-environment)
  - [Hosting Your DID Document](#hosting-your-did-document)
  - [Same Domain with Multiple WebVH DIDs](#same-domain-with-multiple-webvh-dids)
- [Check Environment Setup](#check-environment-setup)
- [LKMV Commands](#lkmv-commands)
- [Backup and Restore Configurations](#backup-and-restore-configurations)
  - [Backup Configurations](#backup-configurations)
  - [Restore Configurations](#restore-configurations)
  - [DID Secrets Recovery](#did-secrets-recovery)


## Core Concepts

- **Decentralised Identifiers (DIDs)** - A globally unique identifier that enables secure digital interactions. The DID is the cornerstone of Self-Sovereign Identity (SSI), a concept that aims to put individuals or entities in control of their digital identities. DID is usually associated with a cryptographic key pair and represented with different DID methods, each with its own benefits. 

- **DIDComm Messaging Protocol** - An open standard for decentralised communication. Built on the foundation of Decentralised Identifiers (DIDs), it enables parties to exchange verifiable data such as credentials and establishes secure communication channels between parties without relying on centralised servers. 

- **Verifiable Credentials (VCs)** - A digital representation of a claim attested by a trusted issuer about the subject (e.g., Individual or Organisation). VC is cryptographically signed and verifiable using cryptographic keys associated with the DID of the issuer. 

- **Personhood Credential (PHC)** – A type of verifiable credential issued by any ecosystem (any qualified entity such as a company, university, nonprofit community, government, etc.) that can attest to the credential holder being a real, unique person within that ecosystem. Part of PHC issuance is providing a verified identity verifiable credential issued by a trusted issuer. 

- **Verifiable Relationship Credential (VRC)** - A type of verifiable credential issued peer-to-peer between holders of personhood credentials to attest to verifiable first-person trust relationships. The verifiable relationship credential validates your personhood credential.


## Decentralised Identity 

The LKMV tool uses the did:webvh to create your Community DID. WebVH is a DID method that enhances the existing did:web method, introducing:  

- Verifiable history, providing a full history of DID document changes. 

- Portability with a self-certifying identifier (SCID), allowing you to move to a different domain. 

- Robust security by introducing a pre-rotation key and witness proof that approves changes to the DID.  

To use the DID method, you must have a publicly available domain name that can host the DID log entries (did.jsonl) to resolve the DID successfully and retrieve the public key information and service endpoints for safe, secure, and private interaction with the community. 

![Sample WebVH DID Method](./docs/assets/didwebvh-sample.png)

## Decentralised Communication 

The LKMV tool seamlessly integrates with any DIDComm-compatible mediator, facilitating secure, private, and decentralised communication using your Community DID. 

A DIDComm mediator plays a crucial role in message delivery while preserving privacy. It handles message routing and storage without ever accessing the message content, which remains encrypted end-to-end between sender and recipient. 
 
![Sample WebVH DID Method](./docs/assets/didcomm-envelopes.png)

When sending a message, it is structured in multiple layers called “envelopes” that provides robust security features, such as confidentiality, sender authenticity, non-repudiation, and sender anonymity. 


## Profiles and Configurations

The tool supports multiple profiles, allowing you to represent different identities across various contexts within your environment.

To use a specific profile when running the tool, set the env variable `LKMV_CONFIG_PROFILE` with the name of your profile, for example:

```bash
export LKMV_CONFIG_PROFILE=profile-1
```

Setting the `LKMV_CONFIG_PROFILE` overrides any value set using the `-p/--profile` option.

Each profile manages two types of configurations:

### Public Configuration

Stored in JSON format, the public configuration contains environment-specific details such as:

- Community DID.
- Mediator DID.
- Security mode (e.g., Unlock Codes or Hardware Token).
- Encrypted private data containing known contacts and relationships.

Config file location:

- Default profile: `~/.config/lkmv/config.json`
- Named profiles: `~/.config/lkmv/config-<PROFILE_NAME>.json`

You can change the default location where the public configuration is saved by setting the env variable `LKMV_CONFIG_PATH` with the new path, for example. 

```bash
export LKMV_CONFIG_PATH=~/.config/lkmv-tool
```

### Private Configuration

An encrypted configuration stored inside the public configuration file, containing sensitive information about:

- List of contacts with their Community DIDs and Alias.
- List of relationships with their:
  - Remote and local Relationship DIDs (R-DIDs)
  - Remove and local Community DIDs (C-DIDs)
  - Relationship aliases



### Secured Configuration

A sensitive information stored in the operating system’s secure storage, e.g., macOS Keychain or Linux Keyring.

The secured configuration includes:

- Private key materials.
- Encrypted Session Key (ESK), if using a hardware token.

If your profile uses a hardware token, the secured data is encrypted using the ESK.

For more details about secured configuration, refer to the [Handling Secured Configuration](./docs/handling-secured-configuration.md) documentation.


## Prerequisites

1. Rust version 1.88 or higher (Install [Rust](https://rust-lang.org/learn/get-started/)
   if needed)
2. Set any environment variables as needed
   - `LKMV_CONFIG_PATH`: Path to lkmv configuration files (default:
     `~/.config/lkmv/config.json`).
   - `LKMV_CONFIG_PROFILE`: Set a specific configuration profile (defaults to `default`).
    
      **NOTE:** Setting the `LKMV_CONFIG_PROFILE` overrides any value set using the `-p/--profile` option.

## Feature Flags 

LKMV currently support two feature flags: 

- **default:** Currently set to `openpgp-card`. To disable default features, use `--no-default-features` flag on the setup command. 

    ```bash
    lkmv --no-default-features setup
    ```

- **openpgp-card:** Enables support for openpgp-card compatible devices. Set as the default feature. 

## Set Up Environment

1. Install the tool locally from the source. 

```bash
cargo install –path . 
```

> **NOTE:** This will change once the tool is published. 

2. Run the setup wizard. 

```bash
lkmv setup 
```

If you wish to setup a different profile instead of **default**, set the `-p/--profile` option when running the setup.

```bash
lkmv -p profile-1 setup 
```

Follow the setup steps to create the configuration, generate your Community DID, and connect to a DIDComm mediator server. 

### Hosting Your DID Document

After running the lkmv setup command, the tool generates a `did.jsonl` file for your Community DID. This DID uses the `did:webvh` method, which requires the DID document to be hosted at a specific URL that matches the DID you configured.

The `did:webvh` method resolves your DID by fetching the DID document from a well-known location on the web. If the document is not hosted at the correct URL, the DID cannot be verified or used.

**For example:**

- If your configured URL is `https://mydomain.com`, you must host the file at:
  
  ```
  https://mydomain.com/.well-known/did.jsonl
  ```

- If your configured URL is `https://mydomain.com/profile1`, you must host the file at: 

  ```
  https://mydomain.com/profile1/did.jsonl
  ```

**Important Note:** 

- The URL must be publicly accessible so the tool can resolve your Community DID using the `did:webvh` method.
- Ensure the file name and path match exactly as shown above.

### Same Domain with Multiple WebVH DIDs

To create different WebVH DIDs for the same domain name, set the URL during setup to:

```bash
✔ Enter the URL that will host your DID document (e.g., https://<your-domain>.com): https://mydomain.com/profile1
```

The setup wizard creates a WebVH DID with the following value:

```bash
did:webvh:QmeQawCuEQFF28UNKxGcue4tKx3Vyc2bgknCPKKY61gCgh:mydomain.com:profile1
```

The `did:webvh` will resolve into `https://mydomain.com/profile1/did.jsonl` to parse the DID document.

This is helpful when you want to setup multiple profiles with different WebVH DIDs for the same domain hosting the DID documents or when doing testing.


## Check Environment Setup

The LKMV configures your environment to ensure your setup is safe, secure, and private when running the tool. 

To check the status or health of your current environment, run the following command. 

```bash
lkmv status 
```

If you wish to check the status for a specific profile, run the following the command.

```bash
lkmv -p profile-1 status 
```

When successful, it displays

- The tool version.
- Your Community DIDs, and whether your Community DID is resolvable. 
- Configured keys for authentication, encryption, and signing related to the Community and Relationship DIDs.
- List of requested and established relationships.
- Connectivity to the configured DIDComm mediator for sending private messages, such as Relationship and VRC requests.

## LKMV Commands

To run commands from an installed binary:

```bash
lkmv contacts list
```

To run commands from the source without building and installing the binary:

```bash
cargo run -- contacts list
```

Refer to the list of [LKMV Tool Commands](./docs/lkmv-tool-commands.md) documentation for all available commands and options.

## Backup and Restore Configurations

The tool provides functionality to backup your profile configurations, including PGP keys, so you can transfer them to another machine or restore previous settings when needed.

### Backup Configurations

To back up the configuration for your profile, run the following command:

```bash
lkmv export settings --file ~/Downloads/lkmv-export.lkmv --passphrase MyPassphrase
```

The command will:

- Export the default profile configuration, including key materials stored in the OS’s secure storage.
- Save the encrypted backup to `~/Downloads/lkmv-export.lkmv`.
- Encrypt the backup using the passphrase provided.

**Important:** Store the backup file in a secure location for future recovery.

### Restore Configurations

To restore the backup on another machine or recover your previous setup, run the following command:

```bash
lkmv setup import --file ~/Downloads/lkmv-export.lkmv --passphrase MyPassphrase
```

The command will:

- Import the configurations from the backup file.
- Recreate the default profile (if no `--profile` option), including secured configuration stored in the OS’s secure storage.

This process is helpful in use cases, such as:

- Transferring LKMV configuration to a new machine.
- Recovering access after losing the original machine.
- Resetting the LKMV configuration.

### DID Secrets Recovery

To restore the same DID and associated secrets, use the **24-word recovery phrase** generated during the previous setup. This recovery phrase allows you to regenerate the DID secrets and an option to retain the same DID value. To do this:

1. Run the setup command:

```bash
lkmv setup
```
> Optionally, run the setup command with the `--profile` to setup another profile.

2. The first prompt will ask you if you would like to recover your DID secrets using the 24-word recovery phrase, select `yes`.

```bash
? Recover Secrets from 24 word recovery phrase? (y/n) › yes
```

3. Enter the 24-word recovery phrase to generate the DID secrets.

4. After you entered the recovery phrase, one of the steps will ask if you would like to use your existing DID, select `yes`.

This process will allow to retain the same DID and DID secrets, including the DID document.

**Note:** The recovery phrase only restores the DID and its secrets, not the full profile configuration, such as contacts, relationships, and logs.
