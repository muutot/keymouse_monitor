import threading


class Timer:

    def __init__(self):
        self.timers = {}
        self.func_times = {}
        self._id = 0

    def get_id(self):
        self._id += 1
        return self._id

    def timer_cycle(self, interval, function, *args, **kwargs):
        timer_obj = threading.Timer(interval, function).start()  # 每60秒调用一次
        timer_id = self.get_id()
        self.timers[timer_id] = timer_obj

    def stop_timer(self, timer_id):
        if not (timer_obj := self.timers.pop(timer_id)):
            return
        timer_obj.cancel()
