[![Build status](https://github.com/cquintana92/git-switch-user/workflows/ci/badge.svg)](https://github.com/cquintana92/git-switch-user/actions)

# Git switch user

Manage your git identities with ease.

## How to get

You can either grab the [latest release](https://github.com/cquintana92/git-switch-user/releases/latest) or build it yourself:

```
$ cargo build --release
```

Add the binary to your path and you are ready to go!

## Configuration

The program honors the XDG Base Directory Specification, and will store its data in `$XDG_CONFIG_HOME/git-switch-user/`. If the variable is not set, it defaults to `$HOME/.config/git-switch-user/`.  

## How to use

As the binary name starts with `git-`, you can use it as it was a `git` subcommand:

```
$ git switch-user
```

Also, if you add the following alias to your `.gitconfig` you can use it as `git su`:

```
[alias]
    su = switch-user
```

### Help

You can get the available options by invoking the following command:

```
$ git switch-user help

USAGE:
    git-switch-user [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    create    Create a new profile
    delete    Delete a profile
    help      Prints this message or the help of the given subcommand(s)
    list      List the available profiles
    set       Set the current profile
```

### List identities

You can list your existing identities by either invoking `git switch-user` or `git switch-user list`:

```
$ git switch-user       
+----------------+-----------------+----------------------+------+------------------------------------------+-------------------+
| Name           | User            | Email                | Sign | Key                                      | SSH Key           |
+----------------+-----------------+----------------------+------+------------------------------------------+-------------------+
| personal       | My personal     | personal@email.com   | ✓    | AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA | ~/.ssh/id_rsa     |
+----------------+-----------------+----------------------+------+------------------------------------------+-------------------+
| * work-profile | My work account | work@myworkemail.com | ✓    | BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB | ~/.ssh/mywork_key |
+----------------+-----------------+----------------------+------+------------------------------------------+-------------------+
```

The current identity will have a `*` along its name.

### Create identities

You can create a new identity by invoking the following command:

```
$ git switch-user create
```

It will prompt for:

* A profile name
* Git username
* Git email
* Whether you want to sign commits (and in case you do, the fingerprint of your GPG Key)
* Whether you want to use a custom SSH key (and in case you do, the path to the SSH key)

### Set identity

You can set an identity by invoking the following command:

```
$ git switch-user set [--global] <profile_name>
```

### Delete identities

You can delete an identity by invoking the following command:

```
$ git switch-user delete <profile_name>
```

## Attributions

* All the GitHub actions workflow has been copied from the [RipGrep](https://github.com/BurntSushi/ripgrep) repository.
* Based on https://github.com/geongeorge/Git-User-Switch

## License

Dual-licensed under MIT or the [UNLICENSE](https://unlicense.org).
