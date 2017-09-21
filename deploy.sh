#! /bin/bash

mkdir deployed-release

cp -r static templates deployed-release

cargo build --release

cp target/release/simplewiki-rs.exe deployed-release

cat << EOF > deployed-release/start.sh
#! /bin/bash

# Edit this file to make to your needs. Below is an example using sublime text
# to open the markdown directory "c:\Dev\wiki". It will be shown at port 3000.
# The port is optional.

./simplewiki-rs.exe --editor "C:\Program Files\Sublime Text 3\sublime_text.exe" --port 3000 --wiki-root "C:\Dev\wiki"
EOF