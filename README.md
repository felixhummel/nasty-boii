Nasty-Boii finds git repos that have changes that are not yet pushed.

WARNING: This is vibe-coded.


# Usage
By default, `nasty-boii` searches in the working directory.
```
nasty-boii
```

You can search in another directory.
```
nasty-boii /tmp
```

The number of threads default to number of cores.
```
nasty-boii --threads 8
```


# Development
Run nasty-boii against test data
```
cargo run -- -l debug tests/data/
```
