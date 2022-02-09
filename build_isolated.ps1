$mnt = $args[0].Trim()
Copy-Item $mnt/* . -Recurse -Force -Exclude $mnt/target
cargo build --release --features windows_subsystem
Copy-Item target/release/maple_timer_ng.exe $mnt/maple_timer_ng_release.exe
echo Done