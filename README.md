# git-subcopy

A way to include single files or directories from large git
repositories. Think of it as a crappy clone of `git subtree**, but with
the ability to hand-pick out what you want.

## Why?

Pick your poison:

- **I hate monorepos!** => I'm not a huge fan of them personally, and
  this is a good reason why. Sometimes you just want to modify one
  component of something, and not have a submodule linking the entire
  repository for everyone building your code to download. This will
  let you copy separate components from a monorepo, or if you truly
  want to follow the unix philosophy, make a separate repository from
  the components you need and them submodule those in.

- **I love monorepos!** => Good for you! This tool will let you
  selectively include code into your repository while still not making
  it too bloaty. You won't have to make more than one repository even
  though you want to fork some external project, and your users won't
  have to download any submodules.

- **I don't have an opinion on monorepos** => Still, this tool is
  pretty cool and you should try it just because :)

## State?

This is definitely not stable, both the library interface and the CLI
interface are both a little hairy. In general, consider this either an
alpha tool or just a proof of concept. The good news is, since all the
code is being copied over and checked into git, you'll never have to
worry about any loss. You should be able to replace this tool with
another later if it's superseded, as all the data like your base
revision is right there in plain text.

## Usage

Here's an example screencast of me messing around with the tool
minutes after the initial version was completed:

[![asciicast](https://asciinema.org/a/YvB6gN61En5XJKtHb8GaGCU3U.svg)](https://asciinema.org/a/YvB6gN61En5XJKtHb8GaGCU3U)

## Installation

Make sure you have rust, a C compiler, and openssh installed. Then
just invoke the following:

```
cargo install --git https://gitlab.com/jD91mZM2/git-subcopy
```
