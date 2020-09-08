# Wagon

Utility to make dotfiles portable with symbolic links

[![build][badge]](https://github.com/yasuyuky/wagon/actions)

[badge]: https://github.com/yasuyuky/wagon/workflows/build/badge.svg

# Installation

`cargo install --git https://github.com/yasuyuky/wagon`

# Usage

Create the dotfiles in your git-controlled directory.

Place dotfiles in a subdirectory such as the following.

```tree
 dotfiles
 ├── bash
 │  └── .bashrc
 ├── direnv
 │  └── .config
 │     └── direnv
 │        ├── .gitignore
 │        └── direnvrc
 ├── git
 │  ├── .config
 │  │  └── git
 │  │     └── ignore
 │  └── .gitconfig
 ...
 ├── python
 │  ├── .config
 │  │  ├── flake8
 │  │  └── yapf
 │  │     └── style
 │  ├── .pylintrc
 │  └── .pythonstartup
 ├── tmux
 │  └── .tmux.conf
 └── zsh
    └── .zshrc
```

Then execute the following command, and symbolic links to your files will be created in your home.

```console
wagon link bash git tmux ...
```

You can also use the `copy` subcommand to copy files.

```console
wagon link bash git tmux ...
```

# Configure

The `.wagon.toml` file controls the behavior of the command.

## `dest` field

By default, the command links the configuration file under your home. If you want to change this behavior, set the `dest` field.

```toml
"dest" = "/"
```

## `init` field

The `init` field can be used to initialize the application to use each configuration file.

```toml
[[init]]
command = "brew"
args = ["install", "direnv"]
os = "macos"
```

For example, you can use the following command to run the initial configuration.

```
wagon init direnv
```
