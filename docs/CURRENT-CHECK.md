# Current Check — v0.2.0

## Build & test

```bash
cargo build --target-dir src/target
cargo test --target-dir src/target
```

Expected: zero warnings, all 54 tests pass (48 unit, 6 integration).

## Version

```bash
./target/release/code-seek --version
```

Expected: `code-seek 0.2.0`.

## Install

```bash
sudo cp target/release/code-seek /usr/local/bin/code-seek
code-seek --version
```

Expected: `code-seek 0.2.0` from the system path.

## EntityType::Trait and EntityType::Impl

```bash
cat > /tmp/check_types.rs << 'EOF'
trait Animal {
    fn name(&self) -> &str;
    fn speak(&self);
}

struct Dog;

impl Animal for Dog {
    fn name(&self) -> &str { "dog" }
    fn speak(&self) { println!("woof"); }
}

impl Dog {
    fn fetch(&self) {}
}
EOF

./target/release/code-seek scan /tmp/check_types.rs
```

Expected: `Animal` shows with icon 󰜁 (Trait), both `Dog` impl blocks show with icon 󰉺 (Impl).

## follow_symlinks = false (default)

```bash
mkdir -p /tmp/sec/src /tmp/sec/secret
echo 'fn hidden() {}' > /tmp/sec/secret/leak.rs
echo 'fn main() {}' > /tmp/sec/src/main.rs
ln -sf /tmp/sec/secret /tmp/sec/src/escape

./target/release/code-seek scan /tmp/sec/src
```

Expected: only `main.rs` appears; `escape/leak.rs` is NOT scanned.

## follow_symlinks = true (explicit)

```bash
mkdir -p /tmp/sec_cfg/.code-seek
printf '[scan]\nfollow_symlinks = true\n' > /tmp/sec_cfg/.code-seek/config.toml

cd /tmp/sec_cfg
code-seek scan /tmp/sec/src
```

Expected: both `main.rs` and `escape/leak.rs` appear.

## max_file_size_mb enforcement

```bash
mkdir -p /tmp/size_test/.code-seek
printf '[scan]\nmax_file_size_mb = 0.000001\n' > /tmp/size_test/.code-seek/config.toml
echo 'fn main() {}' > /tmp/size_test/main.rs

cd /tmp/size_test
code-seek scan .
```

Expected: stderr shows `warning: skipping 'main.rs'`; stdout shows `0 files  0 entities`.

## Unreadable file warning

```bash
mkdir -p /tmp/perm_test
echo 'fn secret() {}' > /tmp/perm_test/secret.rs
chmod 000 /tmp/perm_test/secret.rs

code-seek scan /tmp/perm_test
chmod 644 /tmp/perm_test/secret.rs
```

Expected: stderr shows `warning: skipping '...'`; stdout shows `0 files  0 entities`.

## --lang filter

```bash
code-seek scan src/ --lang rust      # only Rust files
code-seek scan . --lang qml,cpp      # only QML and C++
code-seek scan src/ --lang qml       # no match
```

Expected: third command shows `0 files  0 entities`.

## ignore_dirs via config

```bash
mkdir -p /tmp/proj/src /tmp/proj/target/debug /tmp/proj/.code-seek
echo 'fn main() {}' > /tmp/proj/src/main.rs
echo 'fn leaked() {}' > /tmp/proj/target/debug/build.rs
printf '[scan]\nignore_dirs = ["target"]\n' > /tmp/proj/.code-seek/config.toml

code-seek scan /tmp/proj
```

Expected: `1 file  1 entity` (only `src/main.rs`).

## Error case

```bash
code-seek scan /nonexistent
echo "exit: $?"
```

Expected: exit code 1, stderr `error: '/nonexistent' does not exist`.

## Scan on this repo

```bash
time code-seek scan src/
```

Expected: 11 files, 114 entities, under 1 s.

## Code invariants

```bash
# No count_loc_range calls in parsers
grep -rn 'count_loc_range' src/lang/

# No intermediate FileBlock/EntityRow buffers
grep -n 'FileBlock\|EntityRow\|blocks\.push\|blocks\.iter' src/scan.rs

# No Clone or Eq on Entity/EntityType
grep -n 'Clone\|Eq' src/model.rs
```

Expected: first two greps return no matches; third grep shows only `PartialEq` on `EntityType`.
