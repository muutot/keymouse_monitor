from datetime import datetime, timedelta


def get_next_minute():
    """获取下一个分钟的时间"""
    now = datetime.now()
    next_minute = now + timedelta(minutes=1)
    return next_minute.replace(second=0, microsecond=0)


def get_next_minute_interval():
    return (get_next_minute() - datetime.now()).total_seconds()


def get_today_str():
    return datetime.now().strftime('%Y-%m-%d')


def check_cross_day(last_day):
    if (today := get_today_str()) != last_day:
        return today
    return None
