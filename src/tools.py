from datetime import datetime, timedelta


def get_next_minute():
    """获取下一个分钟的时间"""
    now = datetime.now()
    next_minute = now + timedelta(minutes=1)
    return next_minute.replace(second=0, microsecond=0)


def get_next_minute_interval():
    return (get_next_minute() - datetime.now()).total_seconds()
