#! /bin/bash

mkdir deployed-release

cp -r static templates deployed-release

cargo build --release

cp target/release/simplewiki-rs.exe deployed-release