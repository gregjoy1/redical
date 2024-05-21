# RediCal iCal fuzz targets

> [!WARNING]
> This project is experimental and is currently a work in progress and is **not** production ready.

This crate provides targets/binaries for fuzzing, primarily with the AFL (American fuzzy lop) fuzzer - see [this section of the Rust fuzzing book for more information](https://rust-fuzz.github.io/book/afl.html).

Run the following to run the fuzzer against all main iCal parsed properties:
```bash
./redical_ical_afl_fuzz_targets/start_afl_fuzz.sh
```

Once a crash or a hang has been found, you can find which specific parser context it affects by running it against the following targets/binaries:
* `EventProperties` - e.g. `cat redical_ical_afl_fuzz_targets/fuzz_results/default/hangs/id:000073,src:004696,time:25281347,execs:141302627,op:havoc,rep:1|target/release/event_properties_afl_fuzz_target`
* `QueryProperties` - e.g. `cat redical_ical_afl_fuzz_targets/fuzz_results/default/hangs/id:000073,src:004696,time:25281347,execs:141302627,op:havoc,rep:1|target/release/query_properties_afl_fuzz_target`

We can copy finds we want to keep into the `redical_ical/tests/fuzz_finds/hangs` directory. We can run assertions against this (see `redical_ical/tests/fuzzing_hang_tests.rs` - currently ignored for now but kept for posterity).
