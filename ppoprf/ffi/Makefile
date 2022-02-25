# Makefile to verify the C bindings.

FFIHDR := include/ppoprf.h
FFILIB := ../../target/debug/libffi.a

all: $(FFIDHR) test_ffi

check: test_ffi
	./test_ffi

clean:
	$(RM) test_ffi test_ffi.o
	$(RM) $(FFIHDR)

# Invoke cargo to generate the bindings and static library.
$(FFILIB) $(FFIHDR): src/lib.rs build.rs Cargo.toml cbindgen.toml
	cargo build

test_ffi.o: src/main.c $(FFIHDR)
	$(CC) -g -Wall -I $(dir $(FFIHDR)) -c -o $@ $<

test_ffi: test_ffi.o $(FFILIB)
	$(CC) -g -Wall -o $@ $^
