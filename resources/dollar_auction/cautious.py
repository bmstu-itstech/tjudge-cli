#!/usr/bin/python3

P = int(input())
n = int(input())
for i in range(n):
    opponent_bid = int(input())
    if opponent_bid >= P:
        print(0, flush=True)
        break
    else:
        print(opponent_bid + 1, flush=True)
