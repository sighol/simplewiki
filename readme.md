# Simplewiki

Simplewiki is a web server that presents a folder as a wiki page. The wiki page
supports viewing both markdown files and images.

![Screenshot](docs/screenshot.png)

## Usage

```
USAGE:
    simplewiki [OPTIONS] [wiki_root]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --editor <editor>    Defaults to subl
    -p, --port <PORT>

ARGS:
    <wiki_root>    Directory to serve. Default: .
```
