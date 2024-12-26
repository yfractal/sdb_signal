require 'sdb_signal'

SdbSignal.setup_signal_handler
SdbSignal.start_scheduler
SdbSignal.sleep_with_gvl
