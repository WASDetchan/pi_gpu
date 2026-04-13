#!/bin/bash
echo l = 0.5, d = 2.0
echo "N_пересечений/N_общ,     Оценка π,    Погрешность, Совпадающие цифры"
for n in 100000000 1000000000 10000000000
do
  for i in {0..10} 
  do
    cargo run --release -- $n
  done
done
