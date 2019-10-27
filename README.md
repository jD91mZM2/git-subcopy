# git-subcopy ![Crates.io](https://img.shields.io/crates/v/git-subcopy)

A way to include single files or directories from large git
repositories. Think of it as a crappy clone of `git subtree`, but with
the ability to hand-pick out what you want.

## How it works

You can add a subcopy to your repository in a similar way to how you
can add a subtree:

```sh
git subcopy add <source url> <rev> <source file> <dest file>
```

Any source file in a repository can be included and mapped to any
destination file. Same with directories.

This will literally clone a bare version the repository temporarily
into a cached folder, and then extract out the path you
selected. After this is done, it saves your configuration into a
`.gitcopies` file similar to `.gitmodules`. The main reason for this
is to keep track of the source revision to rebase your changes onto
later. You can skip step this by replacing `add` with `fetch`.

After you've made modifications to the copied file you may want to
check out the diff or run any other arbitrary git command on top of
it.

```sh
git subcopy shell <source file>
```

will re-clone the relevant configuration from your `.gitcopies` file
and add your changes as unstaged. This lets you run `git diff`, and
any changes you make will be propagated back to the original
repository. The checked out revision won't update, however. So to
rebase, use the following command.

```sh
git subcopy rebase <source file> <new revision>
```

will similarly drop you in a shell with your changes applied, but this
time it's commited and a rebase is started. Continue the rebase using
`git rebase --continue`, fix any conflicts you encounter, retry until
success. The standard git stuff. After you're done and exit the shell,
all your changes are copied back and your new base revision is saved
to the `.gitcopies` file.

## Why this exists

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

## State of the project

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

I recommend installing this project (or any project!) using the Nix
package manager. It will automatically fetch all native dependencies
for you so you only need to run the following.

```sh
nix-env -if https://gitlab.com/jD91mZM2/git-subcopy/-/archive/master.tar.gz
```

Alternatively, you can manually make sure you have rust, a C compiler,
and openssl installed and then use the cargo package manager to fetch
this project.

```
cargo install git-subcopy
```
