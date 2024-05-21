#[cfg(test)]
mod tests {
    use redical_ical::properties::EventProperties;

    use std::str::FromStr;

    // Ignore/skip running this test for now as it will fail, useful to keep for testing
    // performance of future parser optimisations.
    #[ignore]
    #[test]
    fn parse_ical_fuzzing_hang_test() {
        /*
        let message: String = std::fs::read_to_string("./tests/fuzz_finds/hangs/id:000005,src:003038,time:3327034,execs:26454896,op:havoc,rep:2").unwrap();
        dbg!(EventProperties::from_str(message.as_str()));

        let message: String = std::fs::read_to_string("./tests/fuzz_finds/hangs/id:000065,src:004524,time:12952877,execs:78555536,op:havoc,rep:2").unwrap();
        dbg!(EventProperties::from_str(message.as_str()));
        */

        let paths = std::fs::read_dir("./tests/fuzz_finds/hangs/").unwrap();

        for path in paths {
            dbg!(&path);
            let path = path.unwrap().path();

            let message: String = std::fs::read_to_string(&path).unwrap();

            let (done_tx, done_rx) = std::sync::mpsc::channel();

            let handle = std::thread::spawn(move || {
                let _ = EventProperties::from_str(message.as_str());

                done_tx.send(()).expect("Unable to send completion signal");
            });

            match done_rx.recv_timeout(std::time::Duration::from_millis(1000)) {
                Ok(_) => handle.join().expect("Thread panicked"),
                Err(_) => panic!("Thread took too long -- hang file: {}", &path.display()),
            }
        }
    }
}
