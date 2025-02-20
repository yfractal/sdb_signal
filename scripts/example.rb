require 'sdb_signal'

def foo(n)
  if n == 0
    sleep 10000000
  else
    foo(n - 1)
  end
end

Thread.new {
  threads = [Thread.current]
  SdbSignal.register_thread(threads)
  sleep 1
  foo(150)
}


SdbSignal.setup_signal_handler
SdbSignal.start_scheduler([])

sleep 10