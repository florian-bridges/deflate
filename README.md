# deflate
An implementation of the deflate algorithm in C

## analyze bitstream
[source] (https://github.com/billbird/gzstat)
A tool for analyzing gzip/DEFLATE bitstreams

```
python gzstat.py < your_gzip_file.gz
```

For verbose output:
```
python gzstat.py --print-block-codes --decode-blocks < your_gzip_file.gz
```