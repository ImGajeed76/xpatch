# Makefile for building C examples with xpatch

# Detect OS
UNAME_S := $(shell uname -s)

# Library name
LIB_NAME = xpatch_c

# Paths
HEADER_DIR = ..
LIB_DIR = ../../../target/release

# Compiler and flags
CC = gcc
CFLAGS = -Wall -Wextra -I$(HEADER_DIR)

# Platform-specific settings
ifeq ($(UNAME_S),Linux)
    LDFLAGS = -L$(LIB_DIR) -l$(LIB_NAME) -Wl,-rpath,$(LIB_DIR)
    LIB_EXT = so
endif
ifeq ($(UNAME_S),Darwin)
    LDFLAGS = -L$(LIB_DIR) -l$(LIB_NAME) -Wl,-rpath,$(LIB_DIR)
    LIB_EXT = dylib
endif
ifneq (,$(findstring MINGW,$(UNAME_S)))
    LDFLAGS = -L$(LIB_DIR) -l$(LIB_NAME)
    LIB_EXT = dll
endif

# Targets
EXAMPLES = basic

all: $(EXAMPLES)

basic: basic.c $(LIB_DIR)/lib$(LIB_NAME).$(LIB_EXT)
	$(CC) $(CFLAGS) -o basic basic.c $(LDFLAGS)

# Build the Rust library if needed
$(LIB_DIR)/lib$(LIB_NAME).$(LIB_EXT):
	@echo "Building Rust library..."
	@cd .. && cargo build --release

clean:
	rm -f $(EXAMPLES)

run: basic
	./basic

.PHONY: all clean run
