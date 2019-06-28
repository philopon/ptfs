# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd) \
          stage=

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    test -f Cargo.lock || cargo generate-lockfile

    cross build --target $TARGET --release

    case "$TARGET" in
        *windows*) cp target/$TARGET/release/ptfs.exe $stage/;;
        *) cp target/$TARGET/release/ptfs $stage/;;
    esac

    cd $stage

    case "$TARGET" in
        *windows*) zip $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.zip *;;
        *) tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz *;;
    esac
    cd $src

    rm -rf $stage
}

main
