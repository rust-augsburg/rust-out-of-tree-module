# SPDX-License-Identifier: GPL-2.0

# Make sure we use LLVM
LLVM := 1
export LLVM

KDIR ?= /lib/modules/`uname -r`/build

default:
	$(MAKE) -C $(KDIR) M=$$PWD

install: default
	# Remove previously installed module
	sudo rmmod rust_out_of_tree || true
	sudo insmod rust_out_of_tree.ko

modules_install: default
	$(MAKE) -C $(KDIR) M=$$PWD modules_install

# Create rust-analyzer config
rust-analyzer:
	make -C $(KDIR) M=$$PWD rust-analyzer
