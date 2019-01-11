# git-lfs-ipfs

A git-lfs server implementation in Rust using IPFS for storage.

[![Build Status](https://travis-ci.org/sameer/git-lfs-ipfs.svg?branch=master)](https://travis-ci.org/sameer/git-lfs-ipfs)

[![Coverage Status](https://coveralls.io/repos/github/sameer/git-lfs-ipfs/badge.svg?branch=master)](https://coveralls.io/github/sameer/git-lfs-ipfs?branch=master)

## Workflow

### Solely IPFS (not possible at the moment)

```bash
# Do your git lfs stuff
# Add http://localhost:5002/ipfs/<ipfs empty folder hash> as the LFS server
git lfs push
# Manually update url to http://localhost:5002/ipfs/<ipfs new hash>
git push origin master
```

### With IPNS (WIP)

```bash
ipfs gen key myrepokey --type=rsa
ipfs name publish QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn --key=myrepokey
# Do your git lfs stuff
# Add http://localhost:5002/ipns/QmIpnsPeerId as the LFS server
git push origin master
# The ipns key, if available locally, will be used to update the hash
# else only download can be done
```

## Behind the Scenes

### Upload

```bash
ipfs add object --> <object hash>
ipfs name resolve /ipns/<ipns peer id> --> <ipfs hash>
ipfs object patch link <ipfs hash> <object id> <object hash> --> <new ipfs hash>
ipfs name publish /ipns/<ipns peer id> <new ipfs hash>
```

### Verify

```bash
ipfs ls /ipns/<ipns peer id> --> <unixfs links list>
grep <object id> <unixfs links list>
```

### Download

```bash
ipfs get /ipns/<ipns peer id>/<object id>
```
