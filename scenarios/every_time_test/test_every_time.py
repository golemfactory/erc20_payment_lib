import os
import datetime
import time

delay_seconds = 600
delay_delta = datetime.timedelta(seconds=delay_seconds)

last_run = datetime.datetime.now() - delay_delta
# last_run = datetime.datetime.now()

command = "erc20_processor --version"
print(command)
os.system(command)

while True:
    now = datetime.datetime.now()
    if now - last_run > delay_delta:
        last_run = now
        command = f"erc20_processor generate --random-receivers -n 30 -a"
        print(command)
        os.system(command)

        command = f"erc20_processor run"
        print(command)
        os.system(command)
    else:
        print(f"Waiting for {delay_delta - (now - last_run)}")
    time.sleep(5)
