package ppoprf

import "testing"

func TestServer(t *testing.T) {
	_, err := CreateServer()
	if err != nil {
		t.Errorf("CreateServer returned nil: %s", err)
	}
}
