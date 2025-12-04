# LKMV Tool Commands

List of commands and options available from the LKMV Tool.

- [Global Options](#global-options)
  - [-p, --profile](#-p---profile)
  - [-u, --unlock-code](#-u---unlock-code)
  - [-h, --help](#-h---help)
- [lkmv setup](#lkmv-setup)
  - [lkmv setup import](#lkmv-setup-import)
- [lkmv status](#lkmv-status)
- [lkmv export](#lkmv-export)
  - [lkmv export pgp-keys](#lkmv-export-pgp-keys)
  - [lkmv export settings](#lkmv-export-settings)
- [lkmv contacts](#lkmv-contacts)
  - [lkmv contacts add](#lkmv-contacts-add)
  - [lkmv contacts remove](#lkmv-contacts-remove)
  - [lkmv contacts list](#lkmv-contacts-list)
- [lkmv relationships](#lkmv-relationships)
  - [lkmv relationships request](#lkmv-relationships-request)

## Global Options

### -p, --profile

Sets the profile configuration used by the CLI. Use this flag when running the setup wizard to create a new profile.

**IMPORTANT:** `LKMV_CONFIG_PROFILE` overrides the `-p, --prorfile` option if set on your CLI.

```bash
lkmv -p profile-1 setup
```

The setup wizard sets up your environment with the new profile called `profile-1`.

To check the environment status from a specific profile.

```bash
lkmv -p profile-1 status
```

To add a new contact to a specific profile.

```bash
lkmv -p profile-1 contacts add --did did:webvh:...
```

### -u, --unlock-code

If you are using an unlock code to protect your secured configuration, use this flag to specify your unlock codes and skip the prompts that ask for your unlock code each time.

```bash
lkmv -u MyUnlockCodes! status
```

### -h, --help

Prints help information about the command.

```bash
lkmv --help
```

Prints the main help information about lkmv, including available commands.

```bash
lkmv setup --help
```

Prints the main help information for the setup command, including available subcommands and flags.

---

## lkmv setup

Run the setup wizard to set up your environment. It creates configuration profiles, generates a Persona DID, and a cryptographic key pair.

```bash
lkmv setup
```

Runs the setup wizard using the default profile.

```bash
lkmv -p profile-1 setup
```

Runs the setup wizard and creates a `profile-1` profile to save the configurations.

### lkmv setup import

Imports LKMV settings exported from a specific profile into a new profile or machine.

#### Options

- `-f, --file <path_to_file>`

  File containing exported settings [default: export.lkmv].

- `-p, --passphrase <passphrase>`

  Passphrase to decrypt the exported settings.

#### Examples

```bash
lkmv setup import -f ~/Download/export.lkmv
```

Runs the setup wizard and uses exported settings from a specific path.

```bash
lkmv -p profile-1 setup import -f ~/Download/export.lkmv
```

Runs the setup wizard with the `profile-1` profile and imports the settings from a specific path.

---

## lkmv status

Prints information about the environment configuration.

```bash
lkmv status
```

Prints information about the environment from the default or current profile (`LKMV_CONFIG_PROFILE`) of the CLI.

```bash
lkmv -p profile-1 status
```

Prints information about the environments using the `profile-1` profile.

---

## lkmv export

Exports settings and other information from the current environment.

### lkmv export pgp-keys

Exports the first set of keys used in your Persona DID for signing, authentication, and decryption operations.

#### Options

- `-p, --passphrase <passphrase>`

  Passphrase to protect the exported PGP secrets.

- `-u, --user-id <first_name last_name <email@domain>>`
  PGP User ID using the 'NAME <EMAIL_ADDRESS>' format.

#### Examples

```bash
lkmv export pgp-keys
```

Exports PGP keys from the default or current profile (`LKMV_CONFIG_PROFILE`) of the CLI.

```bash
lkmv -p profile-1 export pgp-keys
```

Exports PGP keys from a specific profile.

```bash
lkmv -p profile-1 export pgp-keys -p ExportPassphrase -u '<First Last <email@email.com>>'
```

Exports PGP keys from a specific profile and provides the additional details required to export the PGP keys.

### lkmv export settings

Exports settings that you can import into another machine with LKMV installed.

#### Options

- `-p, --passphrase <passphrase>`

  Passphrase to encrypt the exported settings.

- `-f, --file <file_to_save>`

  File to save the settings [default: export.lkmv].

#### Examples

```bash
lkmv export settings
```

Exports settings from the default or current profile (`LKMV_CONFIG_PROFILE`) of the CLI.

```bash
lkmv -p profile-1 settings
```

Exports settings from a specific profile.

```bash
lkmv -p profile-1 export -p ExportPassphrase -f ~/Downloads/profile-1-settings.lkmv
```

Exports settings from a specific profile and provides the additional parameters to save the information.

---

## lkmv contacts

Manage known contacts with their DIDs.

### lkmv contacts add

Add a known DID contact to the list. Replaces an existing contact if the same DID exists.

#### Options

- `-d, --did <did>`

  DID of the contact to add.

- `-a, --alias <alias>`

  Optional alias for the contact.

- `-s, --skip`

  Skips verifying if the DID is valid.

#### Examples

```bash
lkmv contacts add -d did:webvh:... -a "John Doe"
```

Adds John Doe to the list and verifies if the DID is valid for the default or current profile (`LKMV_CONFIG_PROFILE`) of the CLI.

```bash
lkmv contacts add -d did:webvh:... -a "John Doe" -s
```

Adds John Doe to the list but skips DID verification to the default or current profile (`LKMV_CONFIG_PROFILE`) of the CLI.

```bash
lkmv -p profile-1 contacts add -d did:webvh:... -a "John Doe"
```

Adds John Doe to the specific profile contacts list.

### lkmv contacts remove

Removes an existing contact from the list.

#### Options

- `-d, --did <did>`

  DID of the contact to remove.

- `-a, --alias <alias>`

  Alias of the contact to remove.

#### Examples

```bash
lkmv contacts remove -d did:webvh:...
```

Removes a contact with a given DID from the default or current profile (`LKMV_CONFIG_PROFILE`) of the CLI.

```bash
lkmv contacts remove -a 'John Doe'
```

Removes a contact with a given alias from the default or current profile (`LKMV_CONFIG_PROFILE`) of the CLI.

```bash
lkmv -p profile-1 contacts remove -a 'John Doe'
```

Removes a contact with a given alias from a specific profile.

### lkmv contacts list

List all known contacts.

#### Examples

Removes a contact with a given DID.

```bash
lkmv contacts list
```

List all contacts from the default or current profile (`LKMV_CONFIG_PROFILE`) of the CLI.

```bash
lkmv -p profile-1 contacts list
```

List all contacts from a specific profile.

---

## lkmv relationships

Manage relationships with other DIDs.

### lkmv relationships request

Request for a new verifiable relationship.

#### Options

- `-d, --respondent <respondent>`

  A valid DID or alias of your known contacts as the respondent to this relationship request.

- `-a, --alias <alias>`

  Optional alias for the respondent's DID.

- `-r, --reason <reason>`

  Optional reason for requesting a new verifiable relationship.

- `-g, --generate-did`

  Generates a new local relationship DID for the relationship request.

#### Examples

```bash
lkmv relationships request -d did:webvh:... -r "I want to connect."
```

Sends a relationship request to the DID using the default or current profile (`LKMV_CONFIG_PROFILE`) of the CLI.

```bash
lkmv -p profile-1 relationships request -d did:webvh:... -r "I want to connect."
```

Sends a relationship request to the DID from a specific profile.
