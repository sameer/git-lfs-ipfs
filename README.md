# git-lfs-ipfs

git-lfs server and custom transfer implementations in Rust using IPFS for storage.

[![Build Status](https://travis-ci.org/sameer/git-lfs-ipfs.svg?branch=master)](https://travis-ci.org/sameer/git-lfs-ipfs)

[![Coverage Status](https://coveralls.io/repos/github/sameer/git-lfs-ipfs/badge.svg?branch=master)](https://coveralls.io/github/sameer/git-lfs-ipfs?branch=master)

## Workflow

### Solely IPFS (download only)

#### First time

```bash
# Do your git stuff
# Use http://localhost:5002/ipfs/QmEmptyFolderHash as the LFS server for the first time
git lfs push
# Manually update url to http://localhost:5002/ipfs/QmNewHash
git add .lfsconfig
git commit -m "Update git lfs URL"
git push origin master
```

#### Subsequently

```bash
# Do your git stuff
# Use http://localhost:5002/ipfs/QmCurrentHash as the LFS server for the first time
git lfs push
# Manually update url to http://localhost:5002/ipfs/QmNewHash
git add .lfsconfig
git commit -m "Update git lfs URL"
git push origin master
```

### With IPNS publish (download only)

#### First time

```bash
# Make a key for the first time
ipfs gen key myrepokey --type=rsa
ipfs name publish QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn --key=myrepokey
```

#### Subsequently

```bash
# Do your git stuff
# Use http://localhost:5002/ipns/QmIpnsPeerId as the LFS server
git push origin master
# The ipns key, if available locally, will be used to update the hash
# else only download can be done
```

### With DNSLink (download only)

```bash
# Do your git lfs stuff
# Use http://localhost:5002/ipns/mysite.com as the LFS server
git push origin master
# Manually update DNSLink record to /ipfs/QmNewHash
```

## Behind the Scenes

### Upload

```bash
ipfs add object --> QmObjectHash
ipfs name resolve /ipns/QmPeerId --> QmCurrentHash
ipfs object patch link QmCurrentHash <object id (sha256sum)> QmObjectId --> QmNewHash
ipfs name publish QmNewHash --key=QmPeerId
```

### Verify

```bash
ipfs ls /ipns/QmPeerId --> <unixfs links list>
grep <object id> <unixfs links list>
```

### Download

```bash
ipfs get /ipns/QmPeerId/<object id>
```
