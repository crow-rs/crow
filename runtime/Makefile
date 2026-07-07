CC = cc
CFLAGS = -O2 -Wall -fPIC -Iinclude
AR = ar

SRC = $(wildcard src/*.c)
OBJ = $(SRC:src/%.c=build/%.o)
LIB = lib/libcrow_rt.a

all: $(LIB)

$(LIB): $(OBJ)
	@mkdir -p lib
	$(AR) rcs $@ $^

build/%.o: src/%.c include/crow_rt.h
	@mkdir -p build
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -rf build lib