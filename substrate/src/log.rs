#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use std::{
    println as trace, println as debug, println as info, println as warn, println as error,
};

#[cfg(not(test))]
#[allow(unused_imports)]
pub(crate) use log::{debug, error, info, trace, warn};

pub trait Log {
    fn log(&self);
}
