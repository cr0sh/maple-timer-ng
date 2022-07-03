$mnt = $args[0].Trim()
Copy-Item $mnt/* . -Recurse -Force -Exclude $mnt/target,$mnt/.git
cargo -Z sparse-registry build --release --features windows_subsystem
Copy-Item target/release/maple_timer.exe $mnt/maple_timer_release.exe
echo Done