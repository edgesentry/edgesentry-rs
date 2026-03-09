REPO_ROOT := $(shell cd ../../../.. && pwd)
LIB_DIR   := $(REPO_ROOT)/target/release
INCLUDE   := ../../include
TARGET    := c_integration_test

UNAME := $(shell uname)
ifeq ($(UNAME), Darwin)
    EXTRA_LIBS := -framework Security -framework CoreFoundation
else
    EXTRA_LIBS := -lpthread -ldl
endif

.PHONY: all clean build-rust

all: build-rust $(TARGET)

build-rust:
	cargo build -p edgesentry-bridge --release --manifest-path $(REPO_ROOT)/Cargo.toml

$(TARGET): main.c
	cc -o $@ $< \
	    -I$(INCLUDE) \
	    -L$(LIB_DIR) \
	    -ledgesentry_bridge \
	    $(EXTRA_LIBS)

clean:
	rm -f $(TARGET)
