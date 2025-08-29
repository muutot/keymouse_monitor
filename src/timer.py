import threading


class Timer:

    def __init__(self):
        self.timers = {}
        self._id = 0

    def get_id(self):
        self._id += 1
        return self._id

    def timer_once(self, interval, function, *args, **kwargs):
        # 创建一个非周期性定时器
        timer_id = self.get_id()
        timer_obj = threading.Timer(interval, self.wrap_function, args=(timer_id, function, *args), kwargs=kwargs)
        self.timers[timer_id] = timer_obj
        timer_obj.start()  # 启动定时器
        return timer_id

    def wrap_function(self, timer_id, func, *args, **kwargs):
        self.stop_timer(timer_id)
        func(*args, **kwargs)  # 尝试执行给定的函数

    def stop_timer(self, timer_id):
        if (timer_obj := self.timers.pop(timer_id, None)) is not None:
            timer_obj.cancel()
