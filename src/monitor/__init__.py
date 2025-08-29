from src.timer import Timer
from src.monitor.listen import MonitorListen
from src.monitor.monitor_data import MonitorData
from src.tools import get_next_minute_interval


class Monitor:

    def __init__(self, db):
        self.db = db
        self.listen: MonitorListen = MonitorListen(self)
        self.data: MonitorData = MonitorData(self)
        self.timer = Timer()

    def start(self):
        self.listen.start()
        # 添加定时任务
        self.run_timer()

    def run_timer(self):
        self.data.save_to_db_locked("定时任务触发, ")
        self.timer.timer_once(get_next_minute_interval(), self.run_timer)

    def get_keycounts(self):
        return self.data.get_key_counts()
