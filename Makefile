PROOF ?= cysic_proof_24819500.bin
VK ?= vadcop_final.verkey.bin

REPO ?= cysic-labs/venus_proof_verify_demo
RELEASE_TAG ?= v0.1.0
RELEASE_BASE := https://github.com/$(REPO)/releases/download/$(RELEASE_TAG)
PROOF_ASSET ?= cysic_proof_24819500.bin
VK_ASSET ?= vadcop_final.verkey.bin

.PHONY: download verify help

download:
	curl -fL --retry 3 --retry-delay 2 -o "$(PROOF)" "$(RELEASE_BASE)/$(PROOF_ASSET)"
	curl -fL --retry 3 --retry-delay 2 -o "$(VK)" "$(RELEASE_BASE)/$(VK_ASSET)"

verify:
	cargo run -- "$(PROOF)" "$(VK)"

help:
	@echo "Targets:"
	@echo "  make download"
	@echo "  make verify"
	@echo ""
	@echo "Variables:"
	@echo "  PROOF=/abs/path/to/proof.bin"
	@echo "  VK=/abs/path/to/vadcop_final.verkey.bin"
	@echo "  REPO=cysic-labs/venus_proof_verify_demo"
	@echo "  RELEASE_TAG=v0.1.0"
