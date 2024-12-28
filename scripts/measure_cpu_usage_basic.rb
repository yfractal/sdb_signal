require 'sdb_signal'
def a; b end
def b; c end
def c; d end
def d; e end
def e; sleep 100000 end


thread1 = Thread.new { a }

## wait the thread stars

sleep 1

SdbSignal.setup_signal_handler
SdbSignal.start_scheduler_for_current_thread([thread1])
SdbSignal.sleep_with_gvl
