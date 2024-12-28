require 'sdb_signal'
SdbSignal.setup_signal_handler
SdbSignal.start_scheduler_for_current_thread([])
SdbSignal.sleep_with_gvl
