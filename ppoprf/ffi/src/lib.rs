//! Foreign-function interface to the ppoprf randomness implementation
//!
//! This implements a C api so services can easily embed support.
//!

use ppoprf::ppoprf;

/// Opaque struct acts as a handle to the server implementation.
pub struct RandomnessServer {
    inner: ppoprf::Server,
}

/// Construct a new server instance and return an opaque handle to it.
///
/// The handle must be freed by calling randomness_server_release().
// FIXME: What to pass for the md initialization?
#[no_mangle]
pub extern "C" fn randomness_server_create() -> *mut RandomnessServer {
    let test_mds = vec!["test".into()];
    let inner = ppoprf::Server::new(&test_mds);
    let server = Box::new(RandomnessServer { inner });
    Box::into_raw(server)
}

#[no_mangle]
pub extern "C" fn randomness_server_release(ptr: *mut RandomnessServer) {
    assert!(!ptr.is_null());
    let _server = unsafe {
        Box::from_raw(ptr)
    };
    // _server drops as it goes out of scope
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
