/// Use case is to cut out boilerplate on get the value out of the receivers
#[macro_export]
macro_rules! expect_ok {
    ($arg: expr) => {
        $arg.await
            .expect("failed to receive on rx")
            .expect("result was not ok")
    };
}
