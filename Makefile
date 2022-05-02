CDDL_FILES := $(shell find * -type f -name \*.cddl -a \( \! -path target/\* \))
.PHONY: cddl-check clean

clean:
	rm target/all.cddl

target/all.cddl: $(CDDL_FILES)
	mkdir -p "$(@D)"
	rm target/all.cddl || true
	cat $(CDDL_FILES) >> target/all.cddl

target/bin/cddl:
	cargo install cddl --root target/

cddl-check: target/all.cddl target/bin/cddl
	target/bin/cddl compile-cddl --cddl target/all.cddl
 
