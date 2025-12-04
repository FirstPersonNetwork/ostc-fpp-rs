# LKMV Tool Commands

Complete command reference for the LKMV Tool CLI.

## Table of Contents

- [Quick Reference](#quick-reference)
- [Common Patterns](#common-patterns)
- [Global Options](#global-options)
- [Commands](#commands)
  - [setup](#lkmv-setup)
  - [status](#lkmv-status)
  - [export](#lkmv-export)
  - [contacts](#lkmv-contacts)
  - [relationships](#lkmv-relationships)
  - [tasks](#lkmv-tasks)
  - [vrcs](#lkmv-vrcs)

## Quick Reference

| Command | Description |
|---------|-------------|
| `lkmv setup` | Initialise environment and create profile |
| `lkmv status` | View current configuration |
| `lkmv export` | Export settings or PGP keys |
| `lkmv contacts` | Manage known contacts |
| `lkmv relationships` | Manage relationships with other DIDs |
| `lkmv tasks` | Handle outstanding tasks and messages |
| `lkmv vrcs` | Manage Verifiable Relationship Credentials |

## Common Patterns

### Profile Management

All commands support the `-p, --profile` flag to specify which profile to use:

```bash
lkmv -p <profile-name> <command>
```

**Environment Variable:** Set `LKMV_CONFIG_PROFILE` to override the default profile globally.

### Unlock Code

When using an unlock code to protect secured configuration, use `-u, --unlock-code` to avoid repeated prompts:

```bash
lkmv -u <unlock-code> <command>
```

> **Warning:** This exposes your unlock code to the command line history. Avoid using this unless you are using test profile.

### DID Formats

DIDs should follow the format: `did:webvh:<scid>:<domain>`

Example: `did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com`

---

## Global Options

These options work with all commands:

| Flag | Description |
|------|-------------|
| `-p, --profile <NAME>` | Use a specific profile configuration |
| `-u, --unlock-code <CODE>` | Provide unlock code to skip prompts |
| `-h, --help` | Display help information |

**Examples:**

```bash
# View help for main command
lkmv --help

# View help for specific command
lkmv setup --help

# Use specific profile
lkmv -p profile-1 status

# Use unlock code
lkmv -u MyUnlockCode status
```

---

## Commands

## lkmv setup

Initialise your LKMV environment by creating a profile, generating a Persona DID, and setting up cryptographic keys.

**Usage:**
```bash
lkmv setup
lkmv setup import [OPTIONS]
```

**Examples:**

Setup a default profile:
```bash
lkmv setup
```

Create a named profile:
```bash
lkmv -p profile-1 setup
```

### lkmv setup import

Import previously exported LKMV settings into a new profile or machine.

**Options:**

| Flag | Description | Default |
|------|-------------|---------|
| `-f, --file <PATH>` | Path to exported settings file | `export.lkmv` |
| `-p, --passphrase <PASS>` | Passphrase to decrypt settings | Prompted |

**Examples:**

Import with default filename from the current directory:
```bash
lkmv setup import
```

Import from specific file:
```bash
lkmv setup import -f ~/Downloads/backup.lkmv
```

Import with passphrase:
```bash
lkmv setup import -f ~/Downloads/backup.lkmv -p MyPassphrase
```

Import to named profile:
```bash
lkmv -p new-profile setup import -f ~/Downloads/backup.lkmv
```

---

## lkmv status

Display current environment and configuration information.

**Usage:**
```bash
lkmv status
```

**Examples:**

Check default profile status:
```bash
lkmv status
```

Check specific profile status:
```bash
lkmv -p profile-1 status
```

---

## lkmv export

Export settings or cryptographic keys from your environment.

**Usage:**
```bash
lkmv export pgp-keys [OPTIONS]
lkmv export settings [OPTIONS]
```

### lkmv export pgp-keys

Export the primary PGP keys used in your Persona DID for signing, authentication, and decryption.

**Options:**

| Flag | Description | Required |
|------|-------------|----------|
| `-p, --passphrase <PASS>` | Passphrase to protect exported keys | Yes |
| `-u, --user-id <ID>` | PGP User ID: `"Name <email@domain>"` | Yes |

**Examples:**

Export with interactive prompts:
```bash
lkmv export pgp-keys
```

Export with inline parameters:
```bash
lkmv export pgp-keys -p SecurePass123 -u "John Doe <john@example.com>"
```

Export from specific profile:
```bash
lkmv -p profile-1 export pgp-keys
```

### lkmv export settings

Export settings for importing into another profile or machine.

**Options:**

| Flag | Description | Default |
|------|-------------|---------|
| `-p, --passphrase <PASS>` | Passphrase to encrypt settings | Prompted |
| `-f, --file <PATH>` | Output file path | `export.lkmv` |

**Examples:**

Export to default file:
```bash
lkmv export settings
```

Export to specific location:
```bash
lkmv export settings -f ~/backups/profile-backup.lkmv
```

Export with inline passphrase:
```bash
lkmv export settings -p SecurePass123 -f ~/backups/profile-backup.lkmv
```

---

## lkmv contacts

Manage your list of known DIDs and their aliases.

**Usage:**
```bash
lkmv contacts add [OPTIONS]
lkmv contacts remove [OPTIONS]
lkmv contacts list
```

### lkmv contacts add

Add a new contact or update an existing one. If the DID already exists, it will be replaced.

**Options:**

| Flag | Description | Required |
|------|-------------|----------|
| `-d, --did <DID>` | DID of the contact | Yes |
| `-a, --alias <NAME>` | Human-readable alias | No |
| `-s, --skip` | Skip DID validation | No |

> **Note:** By default, DIDs are verified before adding. Use `--skip` to bypass validation.

**Examples:**

Add contact with verification:
```bash
lkmv contacts add -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com -a "John Doe"
```

Add contact without verification:
```bash
lkmv contacts add -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com -a "John Doe" -s
```

Add contact without alias:
```bash
lkmv contacts add -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com
```

### lkmv contacts remove

Remove a contact by DID or alias.

**Options:**

| Flag | Description | Required |
|------|-------------|----------|
| `-d, --did <DID>` | Remove by DID | One required |
| `-a, --alias <NAME>` | Remove by alias | One required |

> **Note:** Provide either `--did` or `--alias` to remove contact.

**Examples:**

Remove by DID:
```bash
lkmv contacts remove -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com
```

Remove by alias:
```bash
lkmv contacts remove -a "John Doe"
```

### lkmv contacts list

Display all contacts in the current profile.

**Usage:**
```bash
lkmv contacts list
```

**Examples:**

List all contacts:
```bash
lkmv contacts list
```

---

## lkmv relationships

Manage relationships with other DIDs for secure communication and VRC issuance.

**Usage:**
```bash
lkmv relationships request [OPTIONS]
lkmv relationships list
```

> **See also:** [Relationships and VRCs Guide](./relationships-vrcs.md)

### lkmv relationships request

Send a relationship request to another DID.

**Options:**

| Flag | Description | Required |
|------|-------------|----------|
| `-d, --respondent <DID>` | Respondent's DID or alias | Yes |
| `-a, --alias <NAME>` | Alias for the respondent | Yes |
| `-r, --reason <TEXT>` | Reason for the relationship | No |
| `-g, --generate-did` | Generate a local R-DID | No |

> **Tip:** Use `--generate-did` to create a Relationship DID (R-DID) for private channel communication. Without it, your Persona DID (P-DID) will be used.

**Examples:**

Send basic relationship request:
```bash
lkmv relationships request -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com -a "John Doe"
```

Send with reason:
```bash
lkmv relationships request -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com -a "John Doe" -r "Coworker connection"
```

Send with R-DID generation:
```bash
lkmv relationships request -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com -a "John Doe" -g
```

Use contact alias:
```bash
lkmv relationships request -d "JohnD" -a "John Doe" -r "Conference attendee"
```

### lkmv relationships list

Display all relationships and their status.

**Usage:**
```bash
lkmv relationships list
```

**Examples:**

List all relationships:
```bash
lkmv relationships list
```

---

## lkmv tasks

Manage outstanding tasks including messages from the mediator, relationship requests, and VRC requests.

**Usage:**
```bash
lkmv tasks list
lkmv tasks fetch
lkmv tasks remove [OPTIONS]
lkmv tasks interact [OPTIONS]
lkmv tasks clear [OPTIONS]
```

### lkmv tasks list

Display all outstanding tasks.

**Usage:**
```bash
lkmv tasks list
```

**Examples:**

List all tasks:
```bash
lkmv tasks list
```

### lkmv tasks fetch

Retrieve new messages and tasks from the mediator.

**Usage:**
```bash
lkmv tasks fetch
```

**Examples:**

Fetch new tasks:
```bash
lkmv tasks fetch
```

### lkmv tasks remove

Remove a specific task by ID.

**Options:**

| Flag | Description | Required |
|------|-------------|----------|
| `-i, --id <UUID>` | Task ID to remove | Yes |

**Examples:**

Remove specific task:
```bash
lkmv tasks remove --id 50ff0179-6d82-4424-8dab-bdf3b0c24b44
```

### lkmv tasks interact

Interactive CLI manager for fetching and processing tasks (relationship requests, VRC requests, etc.).

**Options:**

| Flag | Description | Required |
|------|-------------|----------|
| `-i, --id <UUID>` | Specific task ID to interact with | No |

**Examples:**

Enter interactive mode to fetch and process all tasks:
```bash
lkmv tasks interact
```

Interact with specific task:
```bash
lkmv tasks interact --id 50ff0179-6d82-4424-8dab-bdf3b0c24b44
```

### lkmv tasks clear

Clear all local tasks and optionally remote messages from the mediator.

**Options:**

| Flag | Description |
|------|-------------|
| `--force` | Skip confirmation prompt |
| `--remote` | Remove remote messages from LKMV Task Queue on mediator |

> **Warning:** This action cannot be undone. All tasks and messages will be permanently deleted.

**Examples:**

Clear with confirmation:
```bash
lkmv tasks clear
```

Force clear without confirmation:
```bash
lkmv tasks clear --force
```

Clear all tasks including remote messages:
```bash
lkmv tasks clear --remote
```

---

## lkmv vrcs

Manage Verifiable Relationship Credentials (VRCs).

**Usage:**
```bash
lkmv vrcs request
lkmv vrcs list
lkmv vrcs show <ID>
lkmv vrcs remove <ID>
```

> **See also:** [Relationships and VRCs Guide](./relationships-vrcs.md#request-verifiable-relationship-credential-vrc)

### lkmv vrcs request

Request a VRC from an established relationship.

**Usage:**
```bash
lkmv vrcs request
```

> **Note:** You must have an [established relationship](./relationships-vrcs.md#establish-relationship) before requesting a VRC. Use interactive prompts to select the relationship and provide credential details.

**Examples:**

Request VRC interactively:
```bash
lkmv vrcs request
```

### lkmv vrcs list

Display all VRCs (both issued and received).

**Usage:**
```bash
lkmv vrcs list
```

**Examples:**

List all VRCs:
```bash
lkmv vrcs list
```

### lkmv vrcs show

Display a specific VRC by ID.

**Usage:**
```bash
lkmv vrcs show <ID>
```

**Examples:**

View specific VRC:
```bash
lkmv vrcs show be85696ebea0e947bde696754be67d640a36b63e1ff9da0c7637c933a6cb469f
```

### lkmv vrcs remove

Remove a VRC from local storage.

**Usage:**
```bash
lkmv vrcs remove <ID>
```

**Examples:**

Remove specific VRC:
```bash
lkmv vrcs remove be85696ebea0e947bde696754be67d640a36b63e1ff9da0c7637c933a6cb469f
```

---
