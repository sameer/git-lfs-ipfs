# git-lfs-ipfs

Working git-lfs custom transfer and extension implementations to use IPFS as storage.

[![Build Status](https://travis-ci.org/sameer/git-lfs-ipfs.svg?branch=master)](https://travis-ci.org/sameer/git-lfs-ipfs)

[![Coverage Status](https://coveralls.io/repos/github/sameer/git-lfs-ipfs/badge.svg?branch=master)](https://coveralls.io/github/sameer/git-lfs-ipfs?branch=master)

## Installation

### Building

```bash
git clone git@github.com:sameer/git-lfs-ipfs.git
cd git-lfs-ipfs
cargo build --release
```

### Packages

None yet!

Add the custom transfer and extensions for IPFS to your `~/.gitconfig`:

```
[lfs "customtransfer.ipfs"]
	path = git-lfs-ipfs-cli
	args = transfer
	concurrent = true
	direction = both
[lfs "extension.ipfs"]
    clean = git-lfs-ipfs-cli clean %f
    smudge = git-lfs-ipfs-cli smudge %f
    priority = 0
```

**Note that git-lfs-ipfs will be enabled by default for all future LFS usage if you enable this.**

## Demo

A demo repository is available to test out your installation: [sameer/git-lfs-ipfs-demo](https://github.com/sameer/git-lfs-ipfs-demo).

## Usage

Use git LFS like you usually do and all subsequent files added in LFS will be added to IPFS.

Currently files already on S3, etc. cannot be read unless you remove the `[lfs "customtransfer.ipfs"]` entry in `~/.gitconfig`, because the IPFS custom transfer overrides your default transfer.
