import collections
import threading
import weakref
from datetime import datetime
from functools import wraps

from src.setting import CONFIG
from src.tools import check_cross_day, get_today_str
from src.type_model import MonitorT


class MonitorData:
    base_counts_today: dict
    incremental_counts: dict
    total_clicks_since_save = 0
    today: str

    def __init__(self, parent):
        self.parent: MonitorT = weakref.proxy(parent)
        self.incremental_counts = collections.defaultdict(int)
        self.today = get_today_str()
        self._init_data()
        self.data_lock = threading.Lock()
        self.on_increase = None

    @property
    def db(self):
        return self.parent.db

    def _init_data(self):
        print("数据加载中...")
        # 初始化时加载当天的历史数据到内存
        self.base_counts_today = collections.defaultdict(int, self.db.get_stats_for_day(self.today))
        if self.base_counts_today:
            print(f"成功从数据库加载了 {self.today} 的基础数据。")
        else:
            print(f"数据库中没有找到 {self.today} 的数据，从零开始。")

    @staticmethod
    def _check_lock(func):
        """这是一个装饰器，用来确保被装饰的方法在持有锁的情况下执行"""

        @wraps(func)
        def wrapper(self, *args, **kwargs):
            with self.data_lock:
                return func(self, *args, **kwargs)

        return wrapper

    @_check_lock
    def save_to_db_locked(self, prefix=""):
        """
        合并内存中的增量数据并保存到数据库。
        注意：此函数假定调用它的上下文已经持有了 data_lock。
        """

        if not self.incremental_counts:
            return

        total_today_counts = self._get_key_count()

        # 2. 将合并后的总数据存回数据库
        self.db.upsert_day_stats(get_today_str(), total_today_counts)

        # 3. 更新内存中的基础数据，并清空增量计数器
        if new_date := check_cross_day(self.today):
            self.base_counts_today.clear()
            self.today = new_date
        else:
            self.base_counts_today = total_today_counts
        self.incremental_counts.clear()
        self.total_clicks_since_save = 0
        print(f"{prefix}数据已合并并保存到数据库。当前时间: {datetime.now()}")

    def increase_count(self, key_name):
        self.incremental_counts[key_name] += 1
        self.total_clicks_since_save += 1

        if self.total_clicks_since_save >= CONFIG.save_threshold:
            self.save_to_db_locked()

        if self.on_increase:
            self.on_increase()

    @_check_lock
    def get_key_counts(self):
        return self._get_key_count()

    def _get_key_count(self):
        # 1. 合并基础数据和增量数据
        total_counts = self.base_counts_today.copy()
        for key, value in self.incremental_counts.items():
            total_counts[key] = total_counts.get(key, 0) + value
        return total_counts
