require 'sdb_signal'

def foo(n)
  if n == 0
    sleep 10000000
  else
    foo(n - 1)
  end
end


threads = []
5.times do |i|
  threads << Thread.new do
    # Homland(RubyChina topis API's avg stack depth is 163)
    foo(150)
  end
end

## wait the thread stars
sleep 1

SdbSignal.setup_signal_handler
SdbSignal.start_scheduler(threads)
SdbSignal.sleep_with_gvl
