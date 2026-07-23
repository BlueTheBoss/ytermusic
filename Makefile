DESTDIR ?=
PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin

.PHONY: all build install uninstall clean

all: build

build:
	cargo build --release

install: build
	install -Dm755 target/release/ytermusic $(DESTDIR)$(BINDIR)/ytermusic

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/ytermusic

clean:
	cargo clean
