# OSTC Tool Commands

Complete command reference for the OSTC CLI tool.

## Table of Contents

- [Quick Reference](#quick-reference)
- [Common Patterns](#common-patterns)
- [Global Options](#global-options)
- [Commands](#commands)
  - [setup](#ostc-setup)
  - [status](#ostc-status)
  - [logs](#ostc-logs)
  - [export](#ostc-export)
  - [contacts](#ostc-contacts)
  - [relationships](#ostc-relationships)
  - [tasks](#ostc-tasks)
  - [vrcs](#ostc-vrcs)

## Quick Reference

| Command             | Description                                |
| ------------------- | ------------------------------------------ |
| `ostc setup`         | Initialise environment and create profile  |
| `ostc status`        | View current configuration                 |
| `ostc logs`          | Display log history                        |
| `ostc export`        | Export settings or PGP keys                |
| `ostc contacts`      | Manage known contacts                      |
| `ostc relationships` | Manage relationships with other DIDs       |
| `ostc tasks`         | Handle outstanding tasks and messages      |
| `ostc vrcs`          | Manage Verifiable Relationship Credentials |

## Common Patterns

### Profile Management

All commands support the `-p, --profile` flag to specify which profile to use:

```bash
ostc -p <profile-name> <command>
```

**Environment Variable:** Set `OSTC_CONFIG_PROFILE` to override the default profile globally.

### Unlock Code

When using an unlock code to protect secured configuration, use `-u, --unlock-code` to avoid repeated prompts:

```bash
ostc -u <unlock-code> <command>
```

> **Warning:** This exposes your unlock code to the command line history. Avoid using this unless you are using a test profile.

### DID Formats

DIDs should follow the format: `did:webvh:<scid>:<domain>`

Example: `did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com`

---

## Global Options

These options work with all commands:

| Flag                       | Description                          |
| -------------------------- | ------------------------------------ |
| `-p, --profile <NAME>`     | Use a specific profile configuration |
| `-u, --unlock-code <CODE>` | Provide unlock code to skip prompts  |
| `-h, --help`               | Display help information             |

**Examples:**

```bash
# View help for main command
ostc --help

# View help for specific command
ostc setup --help

# Use specific profile
ostc -p profile-1 status

# Use unlock code
ostc -u MyUnlockCode status
```

---

## Commands

## ostc setup

Initialise your OSTC environment by creating a profile, generating a Persona DID, and setting up cryptographic keys.

**Usage:**

```bash
ostc setup
ostc setup import [OPTIONS]
```

**Examples:**

Setup a default profile:

```bash
ostc setup
```

Create a named profile:

```bash
ostc -p profile-1 setup
```

### ostc setup import

Import previously exported OSTC settings into a new profile or machine.

**Options:**

| Flag                      | Description                    | Default      |
| ------------------------- | ------------------------------ | ------------ |
| `-f, --file <PATH>`       | Path to exported settings file | `export.ostc` |
| `-p, --passphrase <PASS>` | Passphrase to decrypt settings | Prompted     |

**Examples:**

Import with default filename from the current directory:

```bash
ostc setup import
```

Import from specific file:

```bash
ostc setup import -f ~/Downloads/backup.ostc
```

Import with passphrase:

```bash
ostc setup import -f ~/Downloads/backup.ostc -p MyPassphrase
```

Import to named profile:

```bash
ostc -p new-profile setup import -f ~/Downloads/backup.ostc
```

---

## ostc status

Display current environment and configuration information.

**Usage:**

```bash
ostc status
```

**Examples:**

Check default profile status:

```bash
ostc status
```

Check specific profile status:

```bash
ostc -p profile-1 status
```

---

## ostc logs

Display log history of actions and events within OSTC. Logs include relationship events, contact changes, task operations, vrc operations, and configuration updates.

**Usage:**

```bash
ostc logs
```

> **Note:** By default, the log maintains up to 100 most recent entries. Older entries are automatically removed. You can update this number by updating the public configuration `limit` property.

**Examples:**

View all log entries:

```bash
ostc logs
```

View logs for a specific profile:

```bash
ostc -p profile-1 logs
```

---

## ostc export

Export settings or cryptographic keys from your environment.

**Usage:**

```bash
ostc export pgp-keys [OPTIONS]
ostc export settings [OPTIONS]
```

### ostc export pgp-keys

Export the primary PGP keys used in your Persona DID for signing, authentication, and decryption.

**Options:**

| Flag                      | Description                          | Required |
| ------------------------- | ------------------------------------ | -------- |
| `-p, --passphrase <PASS>` | Passphrase to protect exported keys  | Yes      |
| `-u, --user-id <ID>`      | PGP User ID: `"Name <email@domain>"` | Yes      |

**Examples:**

Export with interactive prompts:

```bash
ostc export pgp-keys
```

Export with inline parameters:

```bash
ostc export pgp-keys -p SecurePass123 -u "John Doe <john@example.com>"
```

Export from specific profile:

```bash
ostc -p profile-1 export pgp-keys
```

### ostc export settings

Export settings for importing into another profile or machine.

**Options:**

| Flag                      | Description                    | Default      |
| ------------------------- | ------------------------------ | ------------ |
| `-p, --passphrase <PASS>` | Passphrase to encrypt settings | Prompted     |
| `-f, --file <PATH>`       | Output file path               | `export.ostc` |

**Examples:**

Export to default file:

```bash
ostc export settings
```

Export to specific location:

```bash
ostc export settings -f ~/backups/profile-backup.ostc
```

Export with inline passphrase:

```bash
ostc export settings -p SecurePass123 -f ~/backups/profile-backup.ostc
```

---

## ostc contacts

Manage your list of known DIDs and their aliases.

**Usage:**

```bash
ostc contacts add [OPTIONS]
ostc contacts remove [OPTIONS]
ostc contacts list
```

### ostc contacts add

Add a new contact or update an existing one. If the DID already exists, it will be replaced.

**Options:**

| Flag                 | Description          | Required |
| -------------------- | -------------------- | -------- |
| `-d, --did <DID>`    | DID of the contact   | Yes      |
| `-a, --alias <NAME>` | Human-readable alias | No       |
| `-s, --skip`         | Skip DID validation  | No       |

> **Note:** By default, DIDs are verified before adding. Use `--skip` to bypass validation.

**Examples:**

Add contact with verification:

```bash
ostc contacts add -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com -a "John Doe"
```

Add contact without verification:

```bash
ostc contacts add -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com -a "John Doe" -s
```

Add contact without alias:

```bash
ostc contacts add -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com
```

### ostc contacts remove

Remove a contact by DID or alias.

**Options:**

| Flag                 | Description     | Required     |
| -------------------- | --------------- | ------------ |
| `-d, --did <DID>`    | Remove by DID   | One required |
| `-a, --alias <NAME>` | Remove by alias | One required |

> **Note:** Provide either `--did` or `--alias` to remove contact.

**Examples:**

Remove by DID:

```bash
ostc contacts remove -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com
```

Remove by alias:

```bash
ostc contacts remove -a "John Doe"
```

### ostc contacts list

Display all contacts in the current profile.

**Usage:**

```bash
ostc contacts list
```

**Examples:**

List all contacts:

```bash
ostc contacts list
```

---

## ostc relationships

Manage relationships with other DIDs for secure communication and VRC issuance.

**Usage:**

```bash
ostc relationships request [OPTIONS]
ostc relationships ping [OPTIONS]
ostc relationships remove [OPTIONS]
ostc relationships list
```

> **See also:** [Relationships and VRCs Guide](./relationships-vrcs.md)

### ostc relationships request

Send a relationship request to another DID.

**Options:**

| Flag                     | Description                 | Required |
| ------------------------ | --------------------------- | -------- |
| `-d, --respondent <DID>` | Respondent's DID or alias   | Yes      |
| `-a, --alias <NAME>`     | Alias for the respondent    | Yes      |
| `-r, --reason <TEXT>`    | Reason for the relationship | No       |
| `-g, --generate-did`     | Generate a local R-DID      | No       |

> **Tip:** Use `--generate-did` to create a Relationship DID (R-DID) for private channel communication. Without it, your Persona DID (P-DID) will be used.

**Examples:**

Send basic relationship request:

```bash
ostc relationships request -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com -a "JohnD"
```

Send with reason:

```bash
ostc relationships request -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com -a "JohnD" -r "Coworker connection"
```

Send with R-DID generation:

```bash
ostc relationships request -d did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com -a "JohnD" -g
```

Use contact alias:

```bash
ostc relationships request -d "JohnD" -a "John Doe" -r "Conference attendee"
```

### ostc relationships ping

Send a trust ping message to test connectivity with an established relationship. The remote recipient must check their messages to respond with a pong.

**Options:**

| Flag                 | Description         | Required |
| -------------------- | ------------------- | -------- |
| `-r, --remote <DID>` | Remote DID or alias | Yes      |

> **Note:** This command requires an established relationship. Check for pong responses using `ostc tasks interact`.

**Examples:**

Ping by DID:

```bash
ostc relationships ping -r did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com
```

Ping by alias:

```bash
ostc relationships ping -r "JohnD"
```

### ostc relationships remove

Remove an existing relationship and all associated VRCs (both issued and received).

**Options:**

| Flag                 | Description         | Required |
| -------------------- | ------------------- | -------- |
| `-r, --remote <DID>` | Remote DID or alias | Yes      |

> **Warning:** This action cannot be undone. All VRCs associated with this relationship will be permanently deleted.

**Examples:**

Remove by DID:

```bash
ostc relationships remove -r did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com
```

Remove by alias:

```bash
ostc relationships remove -r "JohnD"
```

### ostc relationships list

Display all relationships and their status.

**Usage:**

```bash
ostc relationships list
```

**Examples:**

List all relationships:

```bash
ostc relationships list
```

---

## ostc tasks

Manage outstanding tasks including messages from the mediator, relationship requests, and VRC requests.

**Usage:**

```bash
ostc tasks list
ostc tasks fetch
ostc tasks remove [OPTIONS]
ostc tasks interact [OPTIONS]
ostc tasks clear [OPTIONS]
```

### ostc tasks list

Display all outstanding tasks.

**Usage:**

```bash
ostc tasks list
```

**Examples:**

List all tasks:

```bash
ostc tasks list
```

### ostc tasks fetch

Retrieve new messages and tasks from the mediator.

**Usage:**

```bash
ostc tasks fetch
```

**Examples:**

Fetch new tasks:

```bash
ostc tasks fetch
```

### ostc tasks remove

Remove a specific task by ID.

**Options:**

| Flag              | Description       | Required |
| ----------------- | ----------------- | -------- |
| `-i, --id <UUID>` | Task ID to remove | Yes      |

**Examples:**

Remove specific task:

```bash
ostc tasks remove --id 50ff0179-6d82-4424-8dab-bdf3b0c24b44
```

### ostc tasks interact

Interactive CLI manager for fetching and processing tasks (relationship requests, VRC requests, etc.).

**Options:**

| Flag              | Description                       | Required |
| ----------------- | --------------------------------- | -------- |
| `-i, --id <UUID>` | Specific task ID to interact with | No       |

**Examples:**

Enter interactive mode to fetch and process all tasks:

```bash
ostc tasks interact
```

Interact with specific task:

```bash
ostc tasks interact --id 50ff0179-6d82-4424-8dab-bdf3b0c24b44
```

ostcostc tasks clear

Clear all local tasks and optionally remote messages from the mediator.

**Options:**

| Flag       | Description                                            |
| ---------- | ------------------------------------------------------ |
| `--force`  | Skip confirmation prompt                               |
| `--remote` | Remove remote messages from OSTC Task Queue on mediator |

> **Warning:** This action cannot be undone. All tasks and messages will be permanently deleted.

**Examples:**

Clear with confirmation:

```bash
ostc tasks clear
```

Force clear without confirmation:

```bash
ostc tasks clear --force
```

Clear all tasks including remote messages:

```bash
ostc tasks clear --remote
```

---

## ostc vrcs

Manage Verifiable Relationship Credentials (VRCs).

**Usage:**

```bash
ostc vrcs request
ostc vrcs list [OPTIONS]
ostc vrcs show <ID>
ostc vrcs remove <ID>
```

> **See also:** [Relationships and VRCs Guide](./relationships-vrcs.md#request-verifiable-relationship-credential-vrc)

### ostc vrcs request

Request a VRC from an established relationship.

**Usage:**

```bash
ostc vrcs request
```

> **Note:** You must have an [established relationship](./relationships-vrcs.md#establish-relationship) before requesting a VRC. Use interactive prompts to select the relationship and provide credential details.

**Examples:**

Request VRC interactively:

```bash
ostc vrcs request
```

### ostc vrcs list

Display all VRCs (both issued and received). Optionally filter by relationship.

**Options:**

| Flag                 | Description                                  | Required |
| -------------------- | -------------------------------------------- | -------- |
| `-r, --remote <DID>` | Show VRCs for a specific remote DID or alias | No       |

**Usage:**

```bash
ostc vrcs list [OPTIONS]
```

**Examples:**

List all VRCs:

```bash
ostc vrcs list
```

List VRCs for a specific relationship by DID:

```bash
ostc vrcs list -r did:webvh:QmbeaiTRfLnkzWvagfAUUuQ8XymXenxNaLVjctqVLafE7u:example.com
```

List VRCs for a specific relationship by alias:

```bash
ostc vrcs list -r "JohnD"
```

### ostc vrcs show

Display a specific VRC by ID.

**Usage:**

```bash
ostc vrcs show <ID>
```

**Examples:**

View specific VRC:

```bash
ostc vrcs show be85696ebea0e947bde696754be67d640a36b63e1ff9da0c7637c933a6cb469f
```

### ostc vrcs remove

Remove a VRC from local storage.

**Usage:**

```bash
ostc vrcs remove <ID>
```

**Examples:**

Remove specific VRC:

```bash
ostc vrcs remove be85696ebea0e947bde696754be67d640a36b63e1ff9da0c7637c933a6cb469f
```

---
