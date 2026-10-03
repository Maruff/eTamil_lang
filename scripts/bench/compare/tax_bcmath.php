<?php
/* Exact decimals via bcmath — PHP's answer to Python's Decimal, and the
 * closest thing it has to what eTamil guarantees in the language itself.
 *
 * bcmath operates on decimal strings, so every operation parses and reformats.
 * That is the cost being measured, and it is why the int-paisa row beside this
 * one exists. */
$n = (int)$argv[1];

bcscale(2);
$RATE = '0.05';
$BASE = '300000';

$total = '0';
$i = 0;
while ($i < $n) {
    $income = bcadd($BASE, (string)$i);
    $tax = bcmul(bcsub($income, $BASE), $RATE);
    $total = bcadd($total, $tax);
    $i++;
}

echo $total, PHP_EOL;
