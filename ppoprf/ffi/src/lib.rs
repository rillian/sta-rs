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
    let test_mds = vec!["t".into()];
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

#[no_mangle]
pub extern "C" fn randomness_server_eval(ptr: *mut RandomnessServer,
        input: *const [u8; ppoprf::COMPRESSED_POINT_LEN], md_index: usize, verifiable: bool, output: *mut [u8]) {
    assert!(!ptr.is_null());
    let server = unsafe {
        // borrow a reference to the actual server without taking ownership of
        // the containing Box, which the caller continues to hold.
        &Box::from_raw(ptr).inner
    };
    assert!(!input.is_null());
    let point = unsafe {
        let bytes = std::slice::from_raw_parts(input, ppoprf::COMPRESSED_POINT_LEN);
        ppoprf::CompressedRistretto::from_slice(bytes)
    };
    let result = server.eval(&point, md_index, verifiable);

}

#[cfg(test)]
mod tests {
    //! Unit tests for the ppoprf foreign-function interface
    //!
    //! This tests the C-compatible api from Rust for convenience.
    //! Testing it from other langauges is also recommended!

    use crate::*;
    use curve25519_dalek::ristretto::CompressedRistretto;

    #[test]
    /// Verify creation/release of the opaque server handle.
    fn unused_instance() {
        let server = randomness_server_create();
        assert!(!server.is_null());
        randomness_server_release(server);
    }

    #[test]
    /// One evaluation call to the ppoprf.
    fn simple_eval() {
        let server = randomness_server_create();
        assert!(!server.is_null());

        // Evaluate a test point.
        let point = CompressedRistretto::default();
        let mut result = ppoprf::Evaluation::default();
        let success = randomness_server_eval(server, point.as_bytes(), 0, false, &mut result);
        randomness_server_release(server);
    }

    #[test]
    /// Verify serialization of internal types.
    fn serialization() {
        let r = CompressedRistretto::default();
        println!("{:?}", r);
    }
}
