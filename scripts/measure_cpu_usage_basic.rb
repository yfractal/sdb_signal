require 'sdb_signal'
SdbSignal.setup_signal_handler
SdbSignal.start_thread
SdbSignal.start_thread
SdbSignal.start_thread
SdbSignal.start_thread
SdbSignal.start_thread

sleep 10000
# SdbSignal.sleep_with_gvl