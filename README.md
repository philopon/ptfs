ptfs
[![Build Status](https://travis-ci.org/philopon/ptfs.svg?branch=master)](https://travis-ci.org/philopon/ptfs)
==
download remote server file via dropbox

features
--
1. dropbox client application is not requred both remote/local.
2. encryption

    * save password: sha256(dropbox user id + ptfs password)
    * encrypto/decripto: aes256_ctr(key=saved password, nonce=0000000000000000)

setup
--
1. login dropbox

    ```.sh
    $ ptfs login
    please type token:
    [2019-06-28T00:46:29Z INFO  ptfs::login] logged-in
    ```
    
2. set password for encryption (optional)

    ```.sh
    $ ptfs crypto enable
    type encrypto password:
    [2019-06-28T00:47:46Z INFO  ptfs::crypto] crypto file enabled
    ```

3. start server (local machine)

    ```.sh
    $ ptfs server $DOWNLOAD_DIR
    ```

usage
--
execute download command (remote machine)

```.sh
$ ptfs download file1
[2019-06-28T00:49:13Z INFO  ptfs::download] file1 is uploaded to Dropbox
```

build
--
1. fill app.rs
2. copy app.rs to src/app.rs
3. cargo build --release
