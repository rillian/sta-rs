package main

/*
#cgo LDFLAGS: -L ../../target/debug -lffi -lpthread -ldl -static
#cgo CFLAGS: -I include
#include "ppoprf.h"
*/
import "C"

import (
	"errors"
	"fmt"
	"log"
	"net/http"
	"os"
	"runtime"
	"unsafe"

	// This module must be imported first because of its side effects of
	// seeding our system entropy pool.
	_ "github.com/brave-experiments/nitro-enclave-utils/randseed"

	nitro "github.com/brave-experiments/nitro-enclave-utils"
	"github.com/bwesterb/go-ristretto"
)

var (
	elog = log.New(os.Stderr, "randsrv: ", log.Ldate|log.Ltime|log.LUTC|log.Lshortfile)
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

// Server represents a PPOPRF randomness server instance.
type Server struct {
	raw    *C.RandomnessServer
	noCopy noCopy
}

func serverFinalizer(server *Server) {
	C.randomness_server_release(server.raw)
	server.raw = nil
}

// createServer creates a new PPOPRF randomness server instance.
//
// FIXME Pass in a list of 8-bit tags defining epochs.
// The instance will generate its own secret key.
func createServer() (*Server, error) {
	// FIXME should we runtime.LockOSThread() here?
	raw := C.randomness_server_create()
	if raw == nil {
		return nil, errors.New("Failed to create randomness server")
	}
	server := &Server{raw: raw}
	runtime.SetFinalizer(server, serverFinalizer)
	return server, nil
}

func getRandomnessHandler() http.HandlerFunc {
	srv, err := createServer()
	if err != nil {
		elog.Fatalf("Failed to create randomness server: %s", err)
	}
	var c ristretto.Point
	var md uint8 = 0
	var verifiable bool
	var output [32]byte

	return func(w http.ResponseWriter, r *http.Request) {
		c.Rand()
		input, err := c.MarshalBinary()
		if err != nil {
			elog.Fatalf("Failed to marshal Ristretto point: %s", err)
		}

		C.randomness_server_eval(srv.raw,
			(*C.uint8_t)(unsafe.Pointer(&input[0])),
			(C.ulong)(md),
			(C.bool)(verifiable),
			(*C.uint8_t)(unsafe.Pointer(&output[0])))
		fmt.Fprintf(w, "%x", output)
	}
}

func main() {
	enclave := nitro.NewEnclave(
		&nitro.Config{
			SOCKSProxy: "socks5://127.0.0.1:1080",
			FQDN:       "nitro.nymity.ch",
			Port:       8080,
			Debug:      true,
			UseACME:    false,
		},
	)
	enclave.AddRoute(http.MethodGet, "/randomness", getRandomnessHandler())
	if err := enclave.Start(); err != nil {
		elog.Fatalf("Enclave terminated: %v", err)
	}
}
