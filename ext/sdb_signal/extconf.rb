# frozen_string_literal: true

require "mkmf"
require "rb_sys/mkmf"

create_rust_makefile("sdb_signal/sdb_signal")
