LIBS=-L lib

.PHONY: fragments deps test docs examples

fragments: lib
	rustc $(LIBS) --opt-level=3 src/lib.rs --out-dir lib/

test:
	rustc $(LIBS) --opt-level=3 --test src/lib.rs -o fragments-test
	./fragments-test --test --bench

docs:
	rustdoc $(LIBS) src/lib.rs

lib:
	mkdir lib