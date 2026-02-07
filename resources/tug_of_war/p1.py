#!/usr/bin/python3

from random import randint

energy = int(input())
iterations = int(input())

if iterations <= 0:
    exit(0)

spend = min(randint(0, energy // iterations), energy)
energy -= spend
print(spend, flush=True)

for _ in range(iterations - 1):
    opponent_spent = int(input())
    spend = min(opponent_spent + 1, energy)
    energy -= spend
    print(spend, flush=True)

# Считываем последнее уведомление о ходе соперника
input()
