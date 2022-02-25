// golang wrapper for the sta-rs ppoprf randomness service.

package ppoprf

/*
#cgo LDFLAGS: -L ../../target/debug -lffi
#cgo CFLAGS: -I include
#include "ppoprf.h"
*/
import "C"

import (
	"errors"
	"runtime"
)

// Embed an zero-length struct to mark our wrapped structs `noCopy`
//
// Wrapper types should have a corresponding finalizer attached to
// handle releasing the underlying pointer.
//
// NOTE Memory allocated by the Rust library MUST be returned over
// the ffi interface for release. It is critical that no calls to
// free any such pointers are made on the go side. To help enforce
// this, wrappers include an empty member with dummy Lock()/Unlock()
// methods to trigger the mutex copy error in `go vet`.
//
// See https://github.com/golang/go/issues/8005 for further discussion.
type noCopy struct{}

func (*noCopy) Lock()   {}
func (*noCopy) Unlock() {}

// ppoprf randomness server instance
type Server struct {
	raw    *C.RandomnessServer
	noCopy noCopy
}

func serverFinalizer(server *Server) {
	C.randomness_server_release(server.raw)
	server.raw = nil
}

// Create a new ppoprf randomness server instance.
//
// FIXME Pass in a list of 8-bit tags defining epochs.
// The instance will generate its own secret key.
func CreateServer() (*Server, error) {
	// FIXME should we runtime.LockOSThread() here?
	raw := C.randomness_server_create()
	if raw == nil {
		return nil, errors.New("Failed to create randomness server")
	}
	server := &Server{raw: raw}
	runtime.SetFinalizer(server, serverFinalizer)
	return server, nil
}
