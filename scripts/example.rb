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

puts "Sampling interval #{SdbSignal.get_sampling_interval/1000} ns"

SdbSignal.register_current_thread
SdbSignal.setup_signal_handler
SdbSignal.start_scheduler

test(150, 500_000_000)
SdbSignal.print_counter

# AWS ec2 m5.4xlarge
# 1. no sampling
#    7.85665089 s
# 2. sampling rate 1_000 ns
#    7.896461828 s
# 3. sampling rate 100 ns
#    8.241924559 s
# 4. sampling rate 10 ns
#    11.75627734 s
# 5. sampling rate 1 ns
#    81.405792602 s
#
# Records
# Sampling interval 1000 ns
# push thread=140035909543744
# Takes = 7.896461828
# counter = 7_448
#
# Sampling interval 100 ns
# push thread=140365364397888
# Takes = 8.241924559
# counter = 78_921
#
# Sampling interval 10 ns
# push thread=139653939095360
# Takes = 11.75627734
# counter = 823_516
#
# Sampling interval 1 ns
# push thread=139633543661376
# Takes = 81.405792602
# counter = 15_039_869
