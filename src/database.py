# --- 数据库优化: 使用类来封装所有数据库操作 ---
import collections
import json
import sqlite3


class Database:
    def __init__(self, db_file):
        self.db_file = db_file
        # 立即初始化连接，确保线程安全
        self.conn = sqlite3.connect(db_file, check_same_thread=False)
        self._init_db()

    def _init_db(self):
        """初始化数据库，创建表"""
        print("检查数据库表结构...")
        cursor = self.conn.cursor()
        cursor.execute('''
                       CREATE TABLE IF NOT EXISTS daily_stats
                       (
                           date
                           TEXT
                           PRIMARY
                           KEY,
                           data
                           TEXT
                           NOT
                           NULL
                       )
                       ''')
        self.conn.commit()
        print("数据库初始化完成。")

    def get_stats_for_day(self, date_str: str) -> dict:
        """获取指定某一天的数据"""
        cursor = self.conn.cursor()
        cursor.execute("SELECT data FROM daily_stats WHERE date = ?", (date_str,))
        result = cursor.fetchone()
        if result:
            return json.loads(result[0])
        return {}

    def get_stats_for_range(self, start_date: str, end_date: str) -> dict:
        """获取并合并一个日期范围内的数据"""
        cursor = self.conn.cursor()
        cursor.execute("SELECT data FROM daily_stats WHERE date BETWEEN ? AND ?", (start_date, end_date))
        results = cursor.fetchall()

        aggregated_data = collections.defaultdict(int)
        for row in results:
            day_data = json.loads(row[0])
            for key, value in day_data.items():
                aggregated_data[key] += value
        return dict(aggregated_data)

    def upsert_day_stats(self, date_str: str, new_data: dict):
        """更新或插入一天的数据 (Update or Insert)"""
        cursor = self.conn.cursor()
        data_json = json.dumps(new_data)
        cursor.execute("INSERT OR REPLACE INTO daily_stats (date, data) VALUES (?, ?)", (date_str, data_json))
        self.conn.commit()
