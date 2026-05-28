# deflate
An implementation of the deflate algorithm in python, c, and rust

## analyze bitstream

A tool for analyzing gzip/DEFLATE bitstreams [source](https://github.com/billbird/gzstat).

```
python gzstat.py < your_gzip_file.gz
```

For verbose output:
```
python gzstat.py --print-block-codes --decode-blocks < your_gzip_file.gz
```
