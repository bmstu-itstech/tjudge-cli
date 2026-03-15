#!/usr/bin/python3

next_choice = "COOPERATE"
n = int(input())
for i in range(n):
    print(next_choice, flush=True)
    next_choice = input()
