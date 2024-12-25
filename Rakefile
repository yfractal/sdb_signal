# frozen_string_literal: true

require "bundler/gem_tasks"
require "rspec/core/rake_task"

RSpec::Core::RakeTask.new(:spec)

require "rb_sys/extensiontask"

task build: :compile

GEMSPEC = Gem::Specification.load("sdb_signal.gemspec")

RbSys::ExtensionTask.new("sdb_signal", GEMSPEC) do |ext|
  ext.lib_dir = "lib/sdb_signal"
end

task default: %i[compile spec]
