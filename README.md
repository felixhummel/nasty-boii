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
$ cargo run -- -l debug tests/data/
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
     Running `target/debug/nasty-boii -l debug tests/data/`
2025-11-07T19:49:20.222952Z  INFO Starting repository scan search_path=tests/data/ threads=None
2025-11-07T19:49:20.223799Z  INFO Found repository repo_path=tests/data/clean-repo
2025-11-07T19:49:20.223802Z  INFO Found repository repo_path=tests/data/nasty-repo
2025-11-07T19:49:20.240084Z DEBUG Repository is clean repo_path=tests/data/clean-repo
tests/data/nasty-repo
```
