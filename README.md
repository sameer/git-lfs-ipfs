# git-lfs-ipfs

git-lfs custom transfer and extension implementations to use IPFS as storage.

[![Build Status](https://travis-ci.org/sameer/git-lfs-ipfs.svg?branch=master)](https://travis-ci.org/sameer/git-lfs-ipfs)

[![Coverage Status](https://coveralls.io/repos/github/sameer/git-lfs-ipfs/badge.svg?branch=master)](https://coveralls.io/github/sameer/git-lfs-ipfs?branch=master)

## Download Workflow

Same for all types -- the server will transmit objects to git-lfs.

## Upload Workflow

### Solely IPFS (not possible right now)

#### First time

```bash
# Do your git stuff
# Use http://localhost:5002/ipfs/QmEmptyFolderHash as the LFS server
git lfs push
# Manually update LFS server url to http://localhost:5002/ipfs/QmNewHash (NOT POSSIBLE RIGHT NOW)
git add .lfsconfig
git commit -m "Update git lfs URL"
git push origin master
```

#### Subsequently

```bash
# Do your git stuff
git lfs push
# Manually update LFS server url to http://localhost:5002/ipfs/QmNewHash (NOT POSSIBLE RIGHT NOW)
git add .lfsconfig
git commit -m "Update git lfs URL"
git push origin master
```

### With IPNS publish (if key available)

#### First time

```bash
# Make a key for the first time
ipfs gen key myrepokey --type=rsa
ipfs name publish QmUNLLsPACCz1vLxQVkXqqLX5R1X345qqfHbsf67hvA3Nn --key=myrepokey
# Do your git stuff
# Use http://localhost:5002/ipns/QmPeerId as the LFS server
git add .lfsconfig
git commit -m "Update git lfs URL"
git push origin master
```

#### Subsequently

```bash
# Do your git stuff
git push origin master
# The ipns key, if available locally, will be used to update the hash
# else only download can be done
```

### DNSLINK (not possible right now)

#### First time

```bash
# Do your git stuff
# Use http://localhost:5002/ipns/mysite.com as the LFS server for the first time
git lfs push
# Manually update DNSLINK record to /ipfs/QmNewHash (NOT POSSIBLE RIGHT NOW)
git add .lfsconfig
git commit -m "Update git lfs URL"
git push origin master
```

#### Subsequently

```bash
# Do your git stuff
git lfs push
# Manually update DNSLINK record to /ipfs/QmNewHash (NOT POSSIBLE RIGHT NOW)
git add .lfsconfig
git commit -m "Update git lfs URL"
git push origin master
```

## Behind the Scenes (CLI equivalent)

### Upload

```bash
ipfs add object --> QmObjectHash
ipfs name resolve /ipns/QmPeerId --> QmCurrentHash
ipfs object patch link QmCurrentHash <object id (sha256sum)> QmObjectId --> QmNewHash
ipfs name publish QmNewHash --key=QmPeerId
```

### Verify

While the step is optional in the LFS protocol, it helps ensure that uploading the object did actually work for an IPNS publish.

```bash
ipfs ls /ipns/QmPeerId --> <unixfs links list>
grep <object id> <unixfs links list>
```

### Download

```bash
ipfs get /ipns/QmPeerId/<object id>
```
