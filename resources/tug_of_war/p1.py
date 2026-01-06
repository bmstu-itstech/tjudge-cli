#!/usr/bin/python3

from random import randint

m = int(input())
n = int(input())

print(randint(0, m // n))
while True:
    k = int(input())
    if k >= 0:
        planed_to_spend = k + 1
        print(planed_to_spend)
    else:
        n -= 1
