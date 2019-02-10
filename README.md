# git-lfs-ipfs

A [git-lfs](https://git-lfs.github.com/) custom transfer & extension implementation that makes using IPFS for storage easy.

[![Build Status](https://travis-ci.org/sameer/git-lfs-ipfs.svg?branch=master)](https://travis-ci.org/sameer/git-lfs-ipfs)

[![Coverage Status](https://coveralls.io/repos/github/sameer/git-lfs-ipfs/badge.svg?branch=master)](https://coveralls.io/github/sameer/git-lfs-ipfs?branch=master)

## Installation

### Building

```bash
git clone https://github.com/sameer/git-lfs-ipfs
cd git-lfs-ipfs/git-lfs-ipfs-cli
cargo build --release
```

### Packages

None yet!

### Configuration

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

**Note that git-lfs-ipfs will be enabled by default for all future LFS usage if you add these lines to your configuration. Make sure to remove them if you do not wish to do so.**

## Demo

A demo repository is available to test out your installation: [sameer/git-lfs-ipfs-demo](https://github.com/sameer/git-lfs-ipfs-demo). Simply clone it once you configure git-lfs-ipfs and verify that no errors occur.

## Usage

Use git LFS like you usually do and all subsequent files added in LFS will be added to IPFS.

Files already on S3, etc. cannot be read unless you remove the `[lfs "customtransfer.ipfs"]` entry in your `~/.gitconfig`; the custom transfer overrides your default transfer so that a file is never uploaded to a remote server.

