#!/bin/bash
for n in 100000000 1000000000 10000000000
do
  for i in {0..10} 
  do
    cargo run --release -- $n
  done
done
