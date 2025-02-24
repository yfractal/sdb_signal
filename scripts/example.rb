require 'sdb_signal'

def test(stacks_depth, n)
  if stacks_depth > 0
    test(stacks_depth - 1, n)
  else
    t0 = Time.now
    while n > 0
      n -= 1
    end
    t1 = Time.now
    puts "Takes = #{t1 - t0}"
  end
end

# SdbSignal.set_sampling_interval(100_000)
# SdbSignal.set_sampling_interval(10_000)
SdbSignal.set_sampling_interval(1_000)

puts "#{SdbSignal.get_sampling_interval} ns"

SdbSignal.register_current_thread
SdbSignal.setup_signal_handler
SdbSignal.start_scheduler

test(150, 1000_000_000)
SdbSignal.print_counter
